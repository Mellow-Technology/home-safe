use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;

/// Filesystem locations for app data. `HOME_SAFE_DATA_DIR` overrides the
/// platform default (e.g. `~/Library/Application Support/com.mellow.HomeSafe`).
#[derive(Clone, Debug)]
pub struct DataPaths {
    pub root: PathBuf,
}

impl DataPaths {
    pub fn resolve() -> Result<Self> {
        let root = match std::env::var_os("HOME_SAFE_DATA_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => ProjectDirs::from("com", "mellow", "HomeSafe")
                .context("could not determine a home directory")?
                .data_dir()
                .to_path_buf(),
        };
        std::fs::create_dir_all(&root)
            .with_context(|| format!("creating data dir {}", root.display()))?;
        Ok(Self { root })
    }

    pub fn database(&self) -> PathBuf {
        self.root.join("homes.db")
    }

    pub fn images(&self) -> PathBuf {
        self.root.join("images")
    }
}
