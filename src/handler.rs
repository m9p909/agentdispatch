use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub struct JsonResponse<T: serde::Serialize>(pub T);

impl<T: serde::Serialize> IntoResponse for JsonResponse<T> {
    fn into_response(self) -> Response {
        (
            StatusCode::OK,
            Json(self.0),
        )
        .into_response()
    }
}

pub struct HtmlResponse(pub String);

impl IntoResponse for HtmlResponse {
    fn into_response(self) -> Response {
        (
            StatusCode::OK,
            [("content-type", "text/html; charset=utf-8")],
            self.0,
        )
        .into_response()
    }
}

pub fn error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(json!({"error": message})),
    )
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_response_creation() {
        let data = json!({"test": "data"});
        let _resp = JsonResponse(data);
    }

    #[test]
    fn test_html_response_creation() {
        let _resp = HtmlResponse("<h1>Test</h1>".to_string());
    }
}
