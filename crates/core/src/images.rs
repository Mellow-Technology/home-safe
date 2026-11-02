use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::store::Store;

/// Downloads a home's images into `<images_dir>/<home_id>/` and records their
/// local paths. Already-downloaded images are skipped, so this is safe to
/// re-run after a partial failure.
#[derive(Clone)]
pub struct ImageDownloader {
    client: reqwest::Client,
    images_dir: PathBuf,
    store: Store,
}

impl ImageDownloader {
    pub fn new(store: Store, images_dir: PathBuf) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("HomeSafe/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { client, images_dir, store })
    }

    /// Returns the number of images newly downloaded.
    pub async fn download_for_home(&self, home_id: i64) -> Result<usize> {
        let dir = self.images_dir.join(home_id.to_string());
        tokio::fs::create_dir_all(&dir).await?;

        let mut downloaded = 0;
        for image in self.store.images(home_id).await? {
            if image.local_path.as_deref().is_some_and(|p| std::path::Path::new(p).exists()) {
                continue;
            }
            let path = dir.join(format!("{:03}.{}", image.position, extension(&image.source_url)));
            match self.fetch(&image.source_url, &path).await {
                Ok(()) => {
                    self.store
                        .set_image_path(image.id, &path.to_string_lossy())
                        .await?;
                    downloaded += 1;
                }
                Err(err) => tracing::warn!(url = %image.source_url, "image download failed: {err:#}"),
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
