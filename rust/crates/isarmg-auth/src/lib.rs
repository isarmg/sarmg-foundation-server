use std::time::{Duration, SystemTime, UNIX_EPOCH};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use thiserror::Error;

type SessionHmac = Hmac<Sha256>;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct SessionIssuer {
    secret: Vec<u8>,
    ttl: Duration,
    secure_cookie: bool,
}

impl SessionIssuer {
    pub fn new(secret: Vec<u8>, ttl: Duration, secure_cookie: bool) -> Result<Self, AuthError> {
        if secret.len() < 32 {
            return Err(AuthError::InvalidInput(
                "session secret must contain at least 32 bytes".into(),
            ));
        }
        Ok(Self {
            secret,
            ttl,
            secure_cookie,
        })
    }

    pub fn issue(&self, subject: &str) -> Result<String, AuthError> {
        if subject.trim().is_empty() {
            return Err(AuthError::InvalidInput("subject is empty".into()));
        }
        let expires = unix_seconds()? + self.ttl.as_secs();
        let payload = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(subject.as_bytes()),
            URL_SAFE_NO_PAD.encode(expires.to_string().as_bytes())
        );
        let signature = self.sign(&payload);
        Ok(format!("{payload}.{signature}"))
    }

    pub fn verify(&self, token: &str) -> Result<String, AuthError> {
        let parts = token.split('.').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(AuthError::Unauthorized);
        }
        let payload = format!("{}.{}", parts[0], parts[1]);
        let expected = self.sign(&payload);
        if !constant_time_eq(expected.as_bytes(), parts[2].as_bytes()) {
            return Err(AuthError::Unauthorized);
        }
        let expires: u64 = URL_SAFE_NO_PAD
            .decode(parts[1])
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|value| value.parse().ok())
            .ok_or(AuthError::Unauthorized)?;
        if expires <= unix_seconds()? {
            return Err(AuthError::Unauthorized);
        }
        URL_SAFE_NO_PAD
            .decode(parts[0])
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .filter(|subject| !subject.trim().is_empty())
            .ok_or(AuthError::Unauthorized)
    }

    pub fn session_cookie(&self, name: &str, token: &str) -> String {
        let mut value = format!(
            "{name}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
            self.ttl.as_secs()
        );
        if self.secure_cookie {
            value.push_str("; Secure");
        }
        value
    }

    pub fn expired_cookie(&self, name: &str) -> String {
        let mut value = format!("{name}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
        if self.secure_cookie {
            value.push_str("; Secure");
        }
        value
    }

    fn sign(&self, payload: &str) -> String {
        let mut mac = SessionHmac::new_from_slice(&self.secret).expect("validated session secret");
        mac.update(payload.as_bytes());
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    }
}

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    if password.len() < 12 {
        return Err(AuthError::InvalidInput(
            "password must contain at least 12 characters".into(),
        ));
    }
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| AuthError::Internal(anyhow::anyhow!(error)))
}

pub fn verify_password(password: &str, encoded: &str) -> bool {
    let Ok(hash) = PasswordHash::new(encoded) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .is_ok()
}

pub fn csrf_token() -> String {
    let nonce: [u8; 32] = {
        use argon2::password_hash::rand_core::OsRng;
        use argon2::password_hash::rand_core::RngCore;
        let mut value = [0u8; 32];
        OsRng.fill_bytes(&mut value);
        value
    };
    URL_SAFE_NO_PAD.encode(nonce)
}

pub fn parse_cookie_token(name: &str, headers: &http::HeaderMap) -> Option<String> {
    headers
        .get(http::header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == name && !value.is_empty()).then(|| value.to_string())
        })
}

fn unix_seconds() -> Result<u64, AuthError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| AuthError::Internal(anyhow::anyhow!(error)))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}
