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

    // Set content-type to text/html unless otherwise specified
    if let Some(content_type) = headers.get("content-type") {
        if content_type.to_str().expect("header is ascii") == "text/plain; charset=utf-8" {
            headers.remove("content-type");
            headers.insert("content-type", HeaderValue::from_str("text/html").expect("text/html is ascii"));
        }
    }
    // Set Cache-Control: no-cache unless otherwise specified. Most endpoints
    // return HTML interpolated with user data which is liable to change all
    // the time, so we don't want these responses to be cached. At least one
    // route, though, does have some specific cache-control. The route to serve
    // static JS can be cached forever.
    if !headers.contains_key("cache-control") {
        headers.insert("cache-control", HeaderValue::from_str("no-cache").expect("no-cache is ascii"));
    };

    Ok(response)
}
