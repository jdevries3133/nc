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
    if let Some(h) = headers.get("content-type") {
        if h.to_str().expect("header is ascii") == "text/plain; charset=utf-8" {
            headers.remove("content-type");
            headers.insert("content-type", HeaderValue::from_str("text/html").expect("text/html is ascii"));
        }
    }

    Ok(response)
}
