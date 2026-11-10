//! Local REST API the browser extension talks to.
//!
//! Binds to loopback only. Requests carrying a web-page `Origin` are rejected
//! so arbitrary sites can't read or write the inventory; only extension
//! origins (and origin-less local tools like curl) get through.

use std::net::{Ipv4Addr, SocketAddr};

use anyhow::Result;
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use home_safe_core::images::ImageDownloader;
use home_safe_core::{Attribute, Home, HomeCapture, HomeImage, Store};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tower_http::cors::{AllowOrigin, CorsLayer};

pub const DEFAULT_PORT: u16 = 47321;

/// Changes the server made, for the UI to react to.
#[derive(Clone, Debug)]
pub enum StoreEvent {
    HomeSaved(i64),
    HomeDeleted(i64),
    ImagesUpdated(i64),
    AttributesUpdated(i64),
}

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub images: ImageDownloader,
    pub events: broadcast::Sender<StoreEvent>,
}

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| is_extension_origin(origin)))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .route("/api/health", get(health))
        .route("/api/homes", get(list_homes).post(save_home))
        .route("/api/homes/lookup", get(lookup_home))
        .route("/api/homes/{id}", get(get_home).delete(delete_home))
        .route("/api/homes/{id}/attributes", put(put_attributes))
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
        .layer(middleware::from_fn(reject_web_origins))
        .layer(cors)
        .with_state(state)
}

pub fn port_from_env() -> u16 {
    std::env::var("HOME_SAFE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

pub async fn serve(state: AppState, port: u16) -> Result<()> {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Home Safe API listening on http://{addr}");
    axum::serve(listener, router(state)).await?;
    Ok(())
}

fn is_extension_origin(origin: &HeaderValue) -> bool {
    origin.to_str().is_ok_and(|o| {
        ["chrome-extension://", "moz-extension://", "safari-web-extension://"]
            .iter()
            .any(|scheme| o.starts_with(scheme))
    })
}

async fn reject_web_origins(req: Request, next: Next) -> Response {
    match req.headers().get(header::ORIGIN) {
        Some(origin) if !is_extension_origin(origin) => {
            (StatusCode::FORBIDDEN, "origin not allowed").into_response()
        }
        _ => next.run(req).await,
    }
}

struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        tracing::error!("request failed: {:#}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{:#}", self.0) })),
        )
            .into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

type ApiResult<T> = std::result::Result<T, ApiError>;

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "app": "home-safe", "version": env!("CARGO_PKG_VERSION") }))
}

async fn list_homes(State(state): State<AppState>) -> ApiResult<Json<Vec<Home>>> {
    Ok(Json(state.store.list_homes().await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LookupQuery {
    source: String,
    source_id: String,
}

#[derive(Serialize)]
struct LookupResponse {
    saved: bool,
    home: Option<Home>,
}

async fn lookup_home(
    State(state): State<AppState>,
    Query(q): Query<LookupQuery>,
) -> ApiResult<Json<LookupResponse>> {
    let home = state.store.find_home(&q.source, &q.source_id).await?;
    Ok(Json(LookupResponse { saved: home.is_some(), home }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveResponse {
    id: i64,
    created: bool,
    image_count: usize,
}

async fn save_home(
    State(state): State<AppState>,
    Json(capture): Json<HomeCapture>,
) -> ApiResult<(StatusCode, Json<SaveResponse>)> {
    let existed = state
        .store
        .find_home(&capture.source, &capture.source_id)
        .await?
        .is_some();
    let id = state.store.save_capture(&capture).await?;
    let _ = state.events.send(StoreEvent::HomeSaved(id));

    // Download in the background so the extension gets an answer right away.
    let bg = state.clone();
    tokio::spawn(async move {
        match bg.images.download_for_home(id).await {
            Ok(0) => {}
            Ok(n) => {
                tracing::info!(home = id, "downloaded {n} images");
                let _ = bg.events.send(StoreEvent::ImagesUpdated(id));
            }
            Err(err) => tracing::warn!(home = id, "image download failed: {err:#}"),
        }
    });

    let status = if existed { StatusCode::OK } else { StatusCode::CREATED };
    Ok((
        status,
        Json(SaveResponse { id, created: !existed, image_count: capture.images.len() }),
    ))
}

#[derive(Serialize)]
struct HomeDetail {
    #[serde(flatten)]
    home: Home,
    images: Vec<HomeImage>,
    attributes: Vec<Attribute>,
}

async fn get_home(State(state): State<AppState>, Path(id): Path<i64>) -> ApiResult<Response> {
    let Some(home) = state.store.get_home(id).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let images = state.store.images(id).await?;
    let attributes = state.store.attributes(id).await?;
    Ok(Json(HomeDetail { home, images, attributes }).into_response())
}

async fn delete_home(State(state): State<AppState>, Path(id): Path<i64>) -> ApiResult<StatusCode> {
    state.images.delete_home(id).await?;
    let _ = state.events.send(StoreEvent::HomeDeleted(id));
    Ok(StatusCode::NO_CONTENT)
}

/// Lets enrichment tools attach data to a home without touching the schema.
async fn put_attributes(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(attrs): Json<Vec<Attribute>>,
) -> ApiResult<StatusCode> {
    if state.store.get_home(id).await?.is_none() {
        return Ok(StatusCode::NOT_FOUND);
    }
    for attr in &attrs {
        state.store.upsert_attribute(id, attr).await?;
    }
    let _ = state.events.send(StoreEvent::AttributesUpdated(id));
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::ServiceExt;

    /// A router over an in-memory store whose images live in a fresh
    /// directory, returned so tests can inspect it.
    async fn app_with_images() -> (Router, std::path::PathBuf) {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("home-safe-test-{}-{n}", std::process::id()));
        let store = Store::open_in_memory().await.unwrap();
        let images = ImageDownloader::new(store.clone(), dir.clone()).unwrap();
        let (events, _) = broadcast::channel(16);
        (router(AppState { store, images, events }), dir)
    }

    async fn app() -> Router {
        app_with_images().await.0
    }

    fn post(origin: Option<&str>) -> Request {
        let mut req = Request::post("/api/homes").header(header::CONTENT_TYPE, "application/json");
        if let Some(o) = origin {
            req = req.header(header::ORIGIN, o);
        }
        req.body(Body::from(r#"{"source":"zillow","sourceId":"42","price":100}"#))
            .unwrap()
    }

    #[tokio::test]
    async fn web_origins_are_rejected() {
        let res = app().await.oneshot(post(Some("https://evil.example"))).await.unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn extension_can_save_and_lookup() {
        let app = app().await;
        let res = app.clone().oneshot(post(Some("chrome-extension://abc"))).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let res = app.clone().oneshot(post(Some("chrome-extension://abc"))).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = app
            .oneshot(
                Request::get("/api/homes/lookup?source=zillow&sourceId=42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["saved"], true);
        assert_eq!(json["home"]["price"], 100);
    }

    #[tokio::test]
    async fn delete_removes_home_and_photos() {
        let (app, dir) = app_with_images().await;
        let res = app.clone().oneshot(post(Some("chrome-extension://abc"))).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let id = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"].as_i64().unwrap();

        let photos = dir.join(id.to_string());
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(photos.join("000.jpg"), b"jpg").unwrap();

        let delete = |origin: &str| {
            Request::delete(format!("/api/homes/{id}"))
                .header(header::ORIGIN, origin)
                .body(Body::empty())
                .unwrap()
        };
        let res = app.clone().oneshot(delete("https://evil.example")).await.unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        assert!(photos.exists());

        let res = app.clone().oneshot(delete("chrome-extension://abc")).await.unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
        assert!(!photos.exists());

        let res = app
            .oneshot(Request::get(format!("/api/homes/{id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        std::fs::remove_dir_all(&dir).ok();
    }
}
