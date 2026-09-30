use axum::{response::IntoResponse, Json};

pub struct ApiError {
    status: u16,
    message: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: 400,
            message: message.into(),
        }
    }

    pub fn too_many(message: impl Into<String>) -> Self {
        Self {
            status: 429,
            message: message.into(),
        }
    }

    pub fn bad_gateway(message: impl Into<String>) -> Self {
        Self {
            status: 502,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = axum::http::StatusCode::from_u16(self.status)
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(serde_json::json!({ "error": self.message }))).into_response()
    }
}
