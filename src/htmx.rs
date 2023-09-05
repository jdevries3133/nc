/// HTMX utils
use axum::http::{HeaderMap, HeaderValue};

pub fn redirect(to: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Hx-Redirect",
        HeaderValue::from_str(to)
            .unwrap_or(HeaderValue::from_str("/").unwrap()),
    );
    headers
}
