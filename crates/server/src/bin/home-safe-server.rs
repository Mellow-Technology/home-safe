//! Headless server: the same API as the desktop app, without the UI.

use anyhow::Result;
use home_safe_core::images::ImageDownloader;
use home_safe_core::paths::DataPaths;
use home_safe_core::Store;
use home_safe_server::{AppState, port_from_env, serve};
use tokio::sync::broadcast;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let paths = DataPaths::resolve()?;
    let store = Store::open(&paths.database()).await?;
    let images = ImageDownloader::new(store.clone(), paths.images())?;
    let (events, _) = broadcast::channel(64);
    serve(AppState { store, images, events }, port_from_env()).await
}
