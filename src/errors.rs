use anyhow::Error;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub struct ServerError(Error);

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        println!("{:?}", self);
        (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong")
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>`
// to turn them into `Result<_, AppError>`. That way you don't need to do that
// manually.
impl<E> From<E> for ServerError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
