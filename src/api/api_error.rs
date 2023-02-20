use crate::error::Error;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub(crate) struct APIError(anyhow::Error);

impl IntoResponse for APIError {
    fn into_response(self) -> Response {
        let any_err = self.0;
        let status = match any_err.downcast_ref::<Error>() {
            Some(Error::AuthForbidden(_, _)) => StatusCode::FORBIDDEN,
            Some(Error::NotImplemented) => StatusCode::NOT_IMPLEMENTED,
            Some(Error::InvalidDNS01) => StatusCode::BAD_REQUEST,
            Some(Error::JsonExtractorRejection(err)) => match err {
                JsonRejection::JsonDataError(_) => StatusCode::UNPROCESSABLE_ENTITY,
                JsonRejection::JsonSyntaxError(_) => StatusCode::BAD_REQUEST,
                JsonRejection::MissingJsonContentType(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(json!({
            "error": format!("{any_err}"),
        }));
        (status, body).into_response()
    }
}

impl<E> From<E> for APIError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
