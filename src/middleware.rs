use super::errors::ServerError;
use axum::{
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
};

pub async fn html_headers<B>(
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, ServerError> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert("content-type", HeaderValue::from_str("text/html")?);

    Ok(response)
}
