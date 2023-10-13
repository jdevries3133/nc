use super::{crypto, models::User};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

/// HMAC-secured session string, signed by $SECRET_KEY
///
/// Note: since this guy is stored in a browser cookie, it's important to
/// esure it does not get too large.
#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub user: User,
}

pub fn serialize_session(session: &Session) -> String {
    let json_bytes = serde_json::to_string(&session)
        .expect("session can be JSON serialized");
    let b64 = general_purpose::STANDARD_NO_PAD.encode(json_bytes);
    let raw_digest = crypto::get_digest(&b64.clone().into_bytes());
    let digest = general_purpose::STANDARD_NO_PAD.encode(raw_digest);
    let session = format!("{}:{}", b64, digest);

    session
}

// We are creating sessions, but not reading from sessions yet.
#[allow(dead_code)]
pub fn deserialize_session(cookie: &str) -> Result<Session, &'static str> {
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
        }
    }

    const SERIALIZED_SESSION: &str =
        "eyJ1c2VyIjp7ImlkIjoxLCJ1c2VybmFtZSI6IkphY2siLCJlbWFpbCI6ImphY2tAamFjay5jb20ifX0:LfHxWjYfG4U7uYkneVf8ZadB3C2z8qV3a8kp1Tnt1sU";

    #[test]
    fn test_serialize_session() {
        env::set_var("SESSION_SECRET", "foo");

        let result = serialize_session(&get_session());
        // little snapshot test
        assert_eq!(result, SERIALIZED_SESSION);
    }

    #[test]
    fn test_deserialize_session() {
        env::set_var("SESSION_SECRET", "foo");

        let result = deserialize_session(&String::from(SERIALIZED_SESSION))
            .expect("result");
        // little snapshot test
        assert_eq!(result.user.id, get_session().user.id);
    }
}
