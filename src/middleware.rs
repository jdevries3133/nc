//! Axum middlewares, modeled as async functions.

use super::{routes::Route, session};
use axum::{
    http::{HeaderValue, Request},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};

/// This will ensure that outgoing requests receive a content-type if the
/// request handler did not specify one. 99% of request handlers in this
/// application are returning a content-type of HTML.
///
/// Note that Axum by default applies a content-type of `text/plain` to outgoing
/// requests. We are going to step on the toes of any _real_ `text/plain`
/// responses on their way out the door, and change this to `text/html`.
///
/// This middleware also ensures that we have `Cache-Control: no-cache` on
/// any responses where cache-control is not specify. This is important because
/// all of my websites run behind Cloudflare, so we need to ensure that
/// we're being explicit about caching.
pub async fn html_headers<B>(request: Request<B>, next: Next<B>) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Set content-type to text/html unless otherwise specified
    if let Some(content_type) = headers.get("content-type") {
        if content_type.to_str().expect("header is ascii")
            == "text/plain; charset=utf-8"
        {
            headers.remove("content-type");
            headers.insert(
                "content-type",
                HeaderValue::from_str("text/html").expect("text/html is ascii"),
            );
        }
    }
    // Set Cache-Control: no-cache unless otherwise specified. Most endpoints
    // return HTML interpolated with user data which is liable to change all
    // the time, so we don't want these responses to be cached. At least one
    // route, though, does have some specific cache-control. The route to serve
    // static JS can be cached forever.
    if !headers.contains_key("cache-control") {
        headers.insert(
            "cache-control",
            HeaderValue::from_str("no-cache").expect("no-cache is ascii"),
        );
    };

    response
}

/// This will validate the session from the request headers and redirect any
/// unauthenticated users to the login route, allowing the creation of a
/// router with protected routes for users only. Unfortunately, this work
/// is not passed along to request handlers because I don't know how, so the
/// session parsing work will be repeated, but these are JWT-style tokens, so
/// validating the session at least does not require a database round trip. This
/// middleware also logs the method, path, and username for authenticated
/// requests.
pub async fn auth<B>(request: Request<B>, next: Next<B>) -> Response {
    let headers = request.headers();
    let session = session::Session::from_headers(headers);
    if let Some(session) = session {
        let path = request.uri().path();
        let method = request.method().as_str();
        let username = session.user.username;
        println!("{method} {path} from {username}");
        next.run(request).await
    } else {
        Redirect::to(&Route::Login.to_string()).into_response()
    }
}
