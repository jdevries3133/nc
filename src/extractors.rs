use super::{models, session};
use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts},
    headers::{HeaderMap, HeaderValue},
    http::{request::Parts, StatusCode},
    response::IntoResponse,
};
use regex::Regex;

pub struct AuthenticatedUser(pub models::User);

fn redirect_to_login() -> (StatusCode, HeaderMap) {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Location",
        HeaderValue::from_str("/authentication/login")
            .expect("that is ascii, I promise"),
    );

    (StatusCode::FOUND, headers)
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        req: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let headers = req.headers;
        let cookie = headers.get("Cookie");
        if cookie.is_none() {
            return Err(redirect_to_login());
        }
        let cookie = cookie.unwrap().to_str().unwrap_or("");
        let re = Regex::new(r"session=(.*)").unwrap();
        let captures = re.captures(cookie);
        if captures.is_none() {
            return Err(redirect_to_login());
        };
        let token = &captures.unwrap()[1];
        let deserialize_result = session::deserialize_session(token);

        if let Ok(session) = deserialize_result {
            Ok(AuthenticatedUser(session.user))
        } else {
            Err(redirect_to_login())
        }
    }
}
