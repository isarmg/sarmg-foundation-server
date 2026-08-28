//! Shared worker-side validation for Union's private `gateway-v1` protocol.
//!
//! A module process is not a standalone HTTP product. Union creates a fresh token for each
//! worker process, injects it through the startup environment and overwrites the matching request
//! headers on every proxied request. Workers validate all four values, including health probes.

use http::{HeaderMap, HeaderValue};
use subtle::ConstantTimeEq;
use thiserror::Error;

pub const PROTOCOL: &str = "gateway-v1";
pub const PROTOCOL_HEADER: &str = "x-union-module-protocol";
pub const AUDIENCE_HEADER: &str = "x-union-module-audience";
pub const TOKEN_HEADER: &str = "x-union-module-token";
pub const PREFIX_HEADER: &str = "x-forwarded-prefix";
pub const PRINCIPAL_HEADER: &str = "x-union-principal";
pub const MAX_PRINCIPAL_BYTES: usize = 128;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PrincipalHeaderError {
    #[error("X-Union-Principal is required")]
    Missing,
    #[error("X-Union-Principal must occur exactly once")]
    Multiple,
    #[error("X-Union-Principal must be valid UTF-8")]
    Encoding,
    #[error(
        "X-Union-Principal must contain 1-{MAX_PRINCIPAL_BYTES} bytes without surrounding whitespace or control characters"
    )]
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatewayIdentity {
    protocol: String,
    audience: String,
    token: String,
    prefix: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GatewayConfigError {
    #[error("UNION_MODULE_PROTOCOL must be gateway-v1")]
    Protocol,
    #[error("UNION_MODULE_AUDIENCE must be {0}")]
    Audience(String),
    #[error("UNION_MODULE_TOKEN must be 64 lowercase hexadecimal characters")]
    Token,
    #[error("UNION_MODULE_PREFIX must be {0}")]
    Prefix(String),
    #[error("{0} is required")]
    Missing(&'static str),
}

impl GatewayIdentity {
    pub fn new(
        protocol: impl Into<String>,
        audience: impl Into<String>,
        token: impl Into<String>,
        prefix: impl Into<String>,
        expected_audience: &str,
        expected_prefix: &str,
    ) -> Result<Self, GatewayConfigError> {
        let identity = Self {
            protocol: protocol.into(),
            audience: audience.into(),
            token: token.into(),
            prefix: prefix.into(),
        };
        if identity.protocol != PROTOCOL {
            return Err(GatewayConfigError::Protocol);
        }
        if identity.audience != expected_audience {
            return Err(GatewayConfigError::Audience(expected_audience.to_owned()));
        }
        if identity.token.len() != 64
            || !identity
                .token
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(GatewayConfigError::Token);
        }
        if identity.prefix != expected_prefix {
            return Err(GatewayConfigError::Prefix(expected_prefix.to_owned()));
        }
        Ok(identity)
    }

    pub fn from_env(
        expected_audience: &str,
        expected_prefix: &str,
    ) -> Result<Self, GatewayConfigError> {
        Self::new(
            required_env("UNION_MODULE_PROTOCOL")?,
            required_env("UNION_MODULE_AUDIENCE")?,
            required_env("UNION_MODULE_TOKEN")?,
            required_env("UNION_MODULE_PREFIX")?,
            expected_audience,
            expected_prefix,
        )
    }

    /// Validate the complete gateway identity. Every comparison is evaluated; the token value is
    /// compared in constant time once its public length has matched.
    pub fn validates(&self, headers: &HeaderMap) -> bool {
        let protocol = header_matches(headers, PROTOCOL_HEADER, &self.protocol);
        let audience = header_matches(headers, AUDIENCE_HEADER, &self.audience);
        let token = header_matches(headers, TOKEN_HEADER, &self.token);
        let prefix = header_matches(headers, PREFIX_HEADER, &self.prefix);
        protocol & audience & token & prefix
    }

    /// Add the identity confirmation required on liveness and readiness responses.
    pub fn apply_health_headers(&self, headers: &mut HeaderMap) {
        headers.insert(
            PROTOCOL_HEADER,
            HeaderValue::from_str(&self.protocol).expect("validated gateway protocol"),
        );
        headers.insert(
            AUDIENCE_HEADER,
            HeaderValue::from_str(&self.audience).expect("validated gateway audience"),
        );
    }

    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    pub fn audience(&self) -> &str {
        &self.audience
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }
}

fn required_env(name: &'static str) -> Result<String, GatewayConfigError> {
    std::env::var(name).map_err(|_| GatewayConfigError::Missing(name))
}

fn header_matches(headers: &HeaderMap, name: &str, expected: &str) -> bool {
    let Some(provided) = headers.get(name).and_then(|value| value.to_str().ok()) else {
        return false;
    };
    provided.len() == expected.len()
        && provided.as_bytes().ct_eq(expected.as_bytes()).unwrap_u8() == 1
}

/// Read the authenticated Union operator forwarded by Core.
///
/// `HeaderValue::to_str` accepts visible ASCII only, while Union usernames are UTF-8. Reading the
/// raw bytes here keeps the wire contract lossless without weakening its canonical-form checks.
pub fn parse_principal(headers: &HeaderMap) -> Result<&str, PrincipalHeaderError> {
    let mut values = headers.get_all(PRINCIPAL_HEADER).iter();
    let value = values.next().ok_or(PrincipalHeaderError::Missing)?;
    if values.next().is_some() {
        return Err(PrincipalHeaderError::Multiple);
    }
    let principal =
        std::str::from_utf8(value.as_bytes()).map_err(|_| PrincipalHeaderError::Encoding)?;
    validate_principal(principal)?;
    Ok(principal)
}

/// Replace any untrusted value with one canonical operator identity.
pub fn insert_principal(
    headers: &mut HeaderMap,
    principal: &str,
) -> Result<(), PrincipalHeaderError> {
    validate_principal(principal)?;
    let value =
        HeaderValue::from_bytes(principal.as_bytes()).map_err(|_| PrincipalHeaderError::Invalid)?;
    headers.insert(PRINCIPAL_HEADER, value);
    Ok(())
}

fn validate_principal(principal: &str) -> Result<(), PrincipalHeaderError> {
    if principal.is_empty()
        || principal.len() > MAX_PRINCIPAL_BYTES
        || principal.trim() != principal
        || principal.chars().any(char::is_control)
    {
        return Err(PrincipalHeaderError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn identity() -> GatewayIdentity {
        GatewayIdentity::new(
            PROTOCOL,
            "photo-backup",
            TOKEN,
            "/api/modules/photo-backup",
            "photo-backup",
            "/api/modules/photo-backup",
        )
        .unwrap()
    }

    fn valid_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(PROTOCOL_HEADER, HeaderValue::from_static(PROTOCOL));
        headers.insert(AUDIENCE_HEADER, HeaderValue::from_static("photo-backup"));
        headers.insert(TOKEN_HEADER, HeaderValue::from_static(TOKEN));
        headers.insert(
            PREFIX_HEADER,
            HeaderValue::from_static("/api/modules/photo-backup"),
        );
        headers
    }

    #[test]
    fn validates_all_four_gateway_values() {
        let identity = identity();
        let mut headers = valid_headers();
        assert!(identity.validates(&headers));
        headers.insert(TOKEN_HEADER, HeaderValue::from_static("wrong"));
        assert!(!identity.validates(&headers));
    }

    #[test]
    fn rejects_noncanonical_configuration() {
        assert_eq!(
            GatewayIdentity::new(
                PROTOCOL,
                "photo-backup",
                TOKEN.to_uppercase(),
                "/api/modules/photo-backup",
                "photo-backup",
                "/api/modules/photo-backup",
            ),
            Err(GatewayConfigError::Token)
        );
    }

    #[test]
    fn health_response_confirms_protocol_and_audience_without_echoing_token() {
        let identity = identity();
        let mut headers = HeaderMap::new();
        identity.apply_health_headers(&mut headers);
        assert_eq!(headers[PROTOCOL_HEADER], PROTOCOL);
        assert_eq!(headers[AUDIENCE_HEADER], "photo-backup");
        assert!(!headers.contains_key(TOKEN_HEADER));
    }

    #[test]
    fn principal_header_round_trips_utf8_and_replaces_untrusted_values() {
        let mut headers = HeaderMap::new();
        headers.append(PRINCIPAL_HEADER, HeaderValue::from_static("attacker-one"));
        headers.append(PRINCIPAL_HEADER, HeaderValue::from_static("attacker-two"));
        assert_eq!(
            parse_principal(&headers),
            Err(PrincipalHeaderError::Multiple)
        );

        insert_principal(&mut headers, "管理员").unwrap();
        assert_eq!(parse_principal(&headers), Ok("管理员"));
        assert_eq!(headers.get_all(PRINCIPAL_HEADER).iter().count(), 1);
    }

    #[test]
    fn principal_header_rejects_noncanonical_or_invalid_values() {
        let mut headers = HeaderMap::new();
        assert_eq!(
            parse_principal(&headers),
            Err(PrincipalHeaderError::Missing)
        );

        for invalid in ["", " admin", "admin ", "admin\troot"] {
            assert_eq!(
                insert_principal(&mut headers, invalid),
                Err(PrincipalHeaderError::Invalid)
            );
        }
        assert_eq!(
            insert_principal(&mut headers, &"a".repeat(MAX_PRINCIPAL_BYTES + 1)),
            Err(PrincipalHeaderError::Invalid)
        );

        headers.insert(
            PRINCIPAL_HEADER,
            HeaderValue::from_bytes(&[0xff]).expect("obs-text is a legal header byte"),
        );
        assert_eq!(
            parse_principal(&headers),
            Err(PrincipalHeaderError::Encoding)
        );
    }
}
