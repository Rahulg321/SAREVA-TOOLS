mod error;
mod guard;
mod llm;
mod tools;

use axum::{
    routing::{get, post},
    Json, Router,
};
use tower_service::Service;
use worker::send::SendWrapper;
use worker::*;

#[derive(Clone)]
pub struct AppState {
    pub env: SendWrapper<Env>,
}

#[event(fetch)]
async fn main(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    Ok(router(env).call(req).await?)
}

fn router(env: Env) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/screen", post(tools::screen::handler))
        .route("/api/returns", post(tools::returns::handler))
        .route(
            "/api/reverse-returns",
            post(tools::reverse_returns::handler),
        )
        .route("/api/debt-capacity", post(tools::debt_capacity::handler))
        .route("/api/comps", post(tools::comps::handler))
        .route("/api/waterfall", post(tools::waterfall::handler))
        .route(
            "/api/ebitda-normalization",
            post(tools::ebitda_normalization::handler),
        )
        .route(
            "/api/statement-metrics",
            post(tools::statement_metrics::handler),
        )
        .route(
            "/api/formula-explain",
            post(tools::formula_explain::handler),
        )
        .route(
            "/api/formula-generate",
            post(tools::formula_generate::handler),
        )
        .route("/api/call-actions", post(tools::call_actions::handler))
        .route("/api/cim-questions", post(tools::cim_questions::handler))
        .route("/api/cim-snapshot", post(tools::cim_snapshot::handler))
        .route("/api/memo", post(tools::memo::handler))
        .with_state(AppState {
            env: SendWrapper::new(env),
        })
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
