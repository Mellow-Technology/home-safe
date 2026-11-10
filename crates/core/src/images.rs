use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::store::Store;

/// Downloads a home's images into `<images_dir>/<home_id>/` and records their
/// local paths. Already-downloaded images are skipped, so this is safe to
/// re-run after a partial failure.
#[derive(Clone)]
pub struct ImageDownloader {
    client: reqwest::Client,
    images_dir: PathBuf,
    store: Store,
    /// Latest download run per home, kept after it finishes so callers can
    /// see the final counts.
    runs: Arc<Mutex<HashMap<i64, Run>>>,
}

struct Run {
    progress: DownloadProgress,
    /// Set when the home is saved again mid-run: its images may have
    /// changed, so the run makes another pass before finishing.
    rerun: bool,
}

/// How far a home's photo download has got.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DownloadProgress {
    /// Photos the home has.
    pub total: usize,
    /// Photos on disk, including ones downloaded by earlier runs.
    pub downloaded: usize,
    /// Photos that failed to download in this run.
    pub failed: usize,
    /// Whether the run has finished.
    pub done: bool,
}

impl ImageDownloader {
    pub fn new(store: Store, images_dir: PathBuf) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("HomeSafe/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { client, images_dir, store, runs: Arc::default() })
    }

    /// Deletes a home's downloaded images from disk.
    pub async fn remove_for_home(&self, home_id: i64) -> Result<()> {
        let dir = self.images_dir.join(home_id.to_string());
        match tokio::fs::remove_dir_all(&dir).await {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                Err(err).with_context(|| format!("removing {}", dir.display()))
            }
            _ => Ok(()),
        }
    }

    /// Progress of the latest download run for a home, if one ran since
    /// the app started.
    pub fn progress(&self, home_id: i64) -> Option<DownloadProgress> {
        self.runs.lock().unwrap().get(&home_id).map(|r| r.progress)
    }

    /// Updates a run's progress; a no-op if the home was deleted meanwhile.
    fn update_progress(&self, home_id: i64, f: impl FnOnce(&mut DownloadProgress)) {
        if let Some(run) = self.runs.lock().unwrap().get_mut(&home_id) {
            f(&mut run.progress);
        }
    }

    /// Deletes a home with its images, both the database rows and the files.
    pub async fn delete_home(&self, home_id: i64) -> Result<()> {
        self.runs.lock().unwrap().remove(&home_id);
        self.store.delete_home(home_id).await?;
        self.remove_for_home(home_id).await
    }

    /// Returns the number of images newly downloaded. Progress is readable
    /// with [`Self::progress`] while this runs and after it finishes. If a
    /// run for this home is already in progress, returns 0 right away and
    /// that run makes another pass once it finishes.
    pub async fn download_for_home(&self, home_id: i64) -> Result<usize> {
        if !self.claim(home_id) {
            return Ok(0);
        }
        self.run_claimed(home_id).await
    }

    /// Marks a download as started, so [`Self::progress`] reports it as
    /// running from now on. Returns false if a run is already in progress;
    /// that run then makes another pass, so nothing is fetched twice at once
    /// and newly saved images aren't missed. When this returns true, call
    /// [`Self::run_claimed`].
    pub fn claim(&self, home_id: i64) -> bool {
        let mut runs = self.runs.lock().unwrap();
        if let Some(run) = runs.get_mut(&home_id).filter(|r| !r.progress.done) {
            run.rerun = true;
            return false;
        }
        runs.insert(home_id, Run { progress: DownloadProgress::default(), rerun: false });
        true
    }

    /// Downloads a home's images after a successful [`Self::claim`].
    pub async fn run_claimed(&self, home_id: i64) -> Result<usize> {
        let mut downloaded = 0;
        loop {
            let result = self.download_images(home_id).await;
            let mut runs = self.runs.lock().unwrap();
            let again = match runs.get_mut(&home_id) {
                Some(run) if result.is_ok() && run.rerun => {
                    run.rerun = false;
                    true
                }
                Some(run) => {
                    run.progress.done = true;
                    false
                }
                None => false, // deleted meanwhile
            };
            downloaded += result?;
            if !again {
                return Ok(downloaded);
            }
        }
    }

    async fn download_images(&self, home_id: i64) -> Result<usize> {
        let dir = self.images_dir.join(home_id.to_string());
        tokio::fs::create_dir_all(&dir).await?;

        let images = self.store.images(home_id).await?;
        let on_disk = |path: Option<&str>| path.is_some_and(|p| std::path::Path::new(p).exists());
        let present = images.iter().filter(|i| on_disk(i.local_path.as_deref())).count();
        self.update_progress(home_id, |p| {
            p.total = images.len();
            p.downloaded = present;
            p.failed = 0;
        });

        let mut downloaded = 0;
        for image in images {
            if on_disk(image.local_path.as_deref()) {
                continue;
            }
            let path = dir.join(format!("{:03}.{}", image.position, extension(&image.source_url)));
            match self.fetch(&image.source_url, &path).await {
                Ok(()) => {
                    self.store
                        .set_image_path(image.id, &path.to_string_lossy())
                        .await?;
                    downloaded += 1;
                    self.update_progress(home_id, |p| p.downloaded += 1);
                }
                Err(err) => {
                    tracing::warn!(url = %image.source_url, "image download failed: {err:#}");
                    self.update_progress(home_id, |p| p.failed += 1);
                }
            }
        }
        Ok(downloaded)
    }

    async fn fetch(&self, url: &str, path: &std::path::Path) -> Result<()> {
        let response = self.client.get(url).send().await?.error_for_status()?;
        let bytes = response.bytes().await?;
        // Write then rename so a crash never leaves a truncated image behind.
        let tmp = path.with_extension("part");
        tokio::fs::write(&tmp, &bytes)
            .await
            .with_context(|| format!("writing {}", tmp.display()))?;
        tokio::fs::rename(&tmp, path).await?;
        Ok(())
    }
}

fn extension(url: &str) -> &'static str {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    if path.ends_with(".png") {
        "png"
    } else if path.ends_with(".webp") {
        "webp"
    } else {
        "jpg"
    }
}
