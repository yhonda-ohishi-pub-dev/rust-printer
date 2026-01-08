use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use super::handlers;
use crate::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(handlers::api_info))
        .route("/health", get(handlers::health_check))
        .route("/generate-pdf", post(handlers::generate_pdf))
        .route("/print-pdf", post(handlers::print_pdf))
        .route("/print", post(handlers::print_file))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
