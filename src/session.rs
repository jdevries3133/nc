//! Cookie-based session, secured by a HMAC signature.
use super::{crypto, models::User};
use axum::headers::{HeaderMap, HeaderValue};
use base64::{engine::general_purpose, Engine as _};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// `Session` is signed and serialized into the `Cookie` header when a
/// [HeaderMap] is passed into the [Session::update_headers()] method. Thus,
/// it's easy to extend this framework to store more information in the secure
/// session cookie by adding fields to this struct. However, keep in mind that
/// since this struct is serialized into a HTTP header, it cannot get too large!
///
/// # Serialization & Deserialization Note
///
/// This struct does derive [Serialize] and [Deserialize]. Internally, these
/// are used to serialize the struct into JSON. Then, the
/// [Session::from_headers()] and [Session::update_headers()] methods perform
/// some additonal ad-hoc serialization and deserialization to grep the session
/// string out of the Cookie string (where it is prefixed by `session=`), and
/// also to convert to/from base64 encoding.
#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub user: User,
    /// Unix seconds timestamp when the token was issued. I'll do token
    /// revocation later.
    pub created_at: u64,
}
impl Session {
    /// Parse the session from request headers, validating the cookie
    /// signature along the way. Returns the [None] variant if the session
    /// header is missing or invalid.
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        let cookie = headers.get("Cookie")?;
        let cookie = cookie.to_str().unwrap_or("");
        let re = Regex::new(r"session=(.*)").unwrap();
        let captures = re.captures(cookie)?;
        let token = &captures[1];
        let deserialize_result = Self::deserialize(token);

        if let Ok(session) = deserialize_result {
            Some(session)
        } else {
            None
        }
    }
    /// Serialize the session into the provided [HeaderMap].
    pub fn update_headers(&self, mut headers: HeaderMap) -> HeaderMap {
        let session_string = self.serialize();
        let header_value =
            format!("session={session_string}; Path=/; HttpOnly");
        headers.insert(
            "Set-Cookie",
            HeaderValue::from_str(&header_value).expect(
                "stringified session can be turned into a header value",
            ),
        );

        headers
    }
    fn serialize(&self) -> String {
        let json_bytes = serde_json::to_string(&self)
            .expect("session can be JSON serialized");
        let b64 = general_purpose::STANDARD_NO_PAD.encode(json_bytes);
        let raw_digest = crypto::get_digest(&b64.clone().into_bytes());
        let digest = general_purpose::STANDARD_NO_PAD.encode(raw_digest);
        let session = format!("{}:{}", b64, digest);

        session
    }
    fn deserialize(cookie: &str) -> Result<Self, &'static str> {
        let parts: Vec<&str> = cookie.split(':').collect();
        if parts.len() != 2 {
            Err("Invalid session")
        } else {
            let b64_json: Vec<u8> = parts[0].into();
            let digest: Vec<u8> =
                match general_purpose::STANDARD_NO_PAD.decode(parts[1]) {
                    Ok(v) => v,
                    Err(_) => {
                        return Err("Cannot base64 decode the digest");
                    }
                };

            if crypto::is_valid(&b64_json, &digest) {
                let json_string =
                    match general_purpose::STANDARD_NO_PAD.decode(b64_json) {
                        Ok(v) => v,
                        Err(_) => {
                            return Err("Cannot base64 decode sesion string");
                        }
                    };

                match serde_json::from_slice(&json_string) {
                    Ok(v) => Ok(v),
                    Err(_) => Err("Cannot deserialize session JSON"),
                }
            } else {
                Err("Failed to validate session signature")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn get_session() -> Session {
        Session {
            user: User {
                id: 1,
                username: "Jack".to_string(),
                email: "jack@jack.com".to_string(),
            },
            created_at: 0,
        }
    }

    const SERIALIZED_SESSION: &str =
    "eyJ1c2VyIjp7ImlkIjoxLCJ1c2VybmFtZSI6IkphY2siLCJlbWFpbCI6ImphY2tAamFjay5jb20ifSwiY3JlYXRlZF9hdCI6MH0:tTtL11Cqgbd3jzCWiPinY8oMJUi6TdqOHlhCIo+gyBk";

    #[test]
    fn test_serialize_session() {
        env::set_var("SESSION_SECRET", "foo");

        let result = &get_session().serialize();
        // little snapshot test
        assert_eq!(result, SERIALIZED_SESSION);
    }

    #[test]
    fn test_deserialize_session() {
        env::set_var("SESSION_SECRET", "foo");

        let result = Session::deserialize(&String::from(SERIALIZED_SESSION))
            .expect("result");
        // little snapshot test
        assert_eq!(result.user.id, get_session().user.id);
    }
}
