//! Idiomatic error handling (claude.md rule #4): Result<T, E> everywhere,
//! explicit propagation, zero panics in handlers.

use actix_web::{http::StatusCode, HttpResponse};
use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: &'a str,
}

impl actix_web::ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Redis(_) | AppError::Db(_) | AppError::Json(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(ErrorBody {
            error: &self.to_string(),
        })
    }
}

impl From<AppError> for tonic::Status {
    fn from(err: AppError) -> Self {
        match err {
            AppError::BadRequest(msg) => tonic::Status::invalid_argument(msg),
            AppError::NotFound(msg) => tonic::Status::not_found(msg),
            other => tonic::Status::internal(other.to_string()),
        }
    }
}
