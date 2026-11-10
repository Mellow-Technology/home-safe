//! Bridges GPUI (its own executor) and the tokio runtime that owns the
//! database, HTTP server and image downloads.

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use home_safe_core::images::ImageDownloader;
use home_safe_core::paths::DataPaths;
use home_safe_core::Store;
use home_safe_server::{AppState, StoreEvent};
use tokio::runtime::Runtime;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct Backend {
    runtime: Arc<Runtime>,
    pub store: Store,
    pub images: ImageDownloader,
    pub events: broadcast::Sender<StoreEvent>,
    pub images_dir: PathBuf,
    pub port: u16,
}

impl Backend {
    pub fn start() -> Result<Self> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("home-safe-io")
                .build()?,
        );
        let paths = DataPaths::resolve()?;
        tracing::info!("data dir: {}", paths.root.display());
        let store = runtime.block_on(Store::open(&paths.database()))?;
        let images = ImageDownloader::new(store.clone(), paths.images())?;
        let (events, _) = broadcast::channel(256);
        let port = home_safe_server::port_from_env();

        let state = AppState { store: store.clone(), images: images.clone(), events: events.clone() };
        runtime.spawn(async move {
            if let Err(err) = home_safe_server::serve(state, port).await {
                tracing::error!("API server stopped: {err:#}");
            }
        });

        Ok(Self { runtime, store, images, events, images_dir: paths.images(), port })
    }

    /// Runs `fut` on the tokio runtime; the returned future can be awaited
    /// from GPUI tasks.
    pub fn run<F, T>(&self, fut: F) -> impl Future<Output = Result<T>> + use<F, T>
    where
        F: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        let handle = self.runtime.spawn(fut);
        async move { handle.await? }
    }
}
