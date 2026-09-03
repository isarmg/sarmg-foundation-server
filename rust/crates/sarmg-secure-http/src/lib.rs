//! Network policy is a closed enum; production code cannot opt into arbitrary plaintext HTTP.

use reqwest::{Client, Response, redirect::Policy};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use url::Url;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkPolicy {
    PublicHttps,
    PrivateDevice {
        allow_loopback: bool,
        allow_link_local: bool,
    },
    LoopbackDevelopment,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResponseBudget {
    pub max_header_bytes: usize,
    pub max_body_bytes: usize,
}
impl Default for ResponseBudget {
    fn default() -> Self {
        Self {
            max_header_bytes: 64 * 1024,
            max_body_bytes: 1024 * 1024,
        }
    }
}

pub struct SecureHttpClient {
    client: Client,
    policy: NetworkPolicy,
    budget: ResponseBudget,
}
impl SecureHttpClient {
    pub fn new(
        policy: NetworkPolicy,
        total_timeout: Duration,
        connect_timeout: Duration,
        budget: ResponseBudget,
    ) -> Result<Self, Error> {
        if total_timeout.is_zero()
            || connect_timeout.is_zero()
            || budget.max_header_bytes == 0
            || budget.max_body_bytes == 0
        {
            return Err(Error::InvalidBudget);
        }
        let mut builder = Client::builder()
            .timeout(total_timeout)
            .connect_timeout(connect_timeout)
            .redirect(Policy::none());
        if matches!(
            policy,
            NetworkPolicy::PrivateDevice { .. } | NetworkPolicy::LoopbackDevelopment
        ) {
            builder = builder.no_proxy();
        }
        Ok(Self {
            client: builder.build()?,
            policy,
            budget,
        })
    }
    pub async fn validate_url(&self, url: &Url) -> Result<Vec<SocketAddr>, Error> {
        if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
            return Err(Error::UnsafeUrl);
        }
        let host = url.host_str().ok_or(Error::UnsafeUrl)?;
        let port = url.port_or_known_default().ok_or(Error::UnsafeUrl)?;
        match self.policy {
            NetworkPolicy::PublicHttps if url.scheme() != "https" => {
                return Err(Error::HttpsRequired);
            }
            NetworkPolicy::PrivateDevice { .. } if !matches!(url.scheme(), "http" | "https") => {
                return Err(Error::UnsafeScheme);
            }
            NetworkPolicy::LoopbackDevelopment if url.scheme() != "http" => {
                return Err(Error::UnsafeScheme);
            }
            _ => {}
        }
        let addresses = tokio::net::lookup_host((host, port))
            .await
            .map_err(Error::Resolve)?
            .collect::<Vec<_>>();
        if addresses.is_empty() {
            return Err(Error::ResolveEmpty);
        }
        for address in &addresses {
            validate_address(self.policy, address.ip())?;
        }
        Ok(addresses)
    }
    pub async fn get_bytes(&self, url: Url) -> Result<Vec<u8>, Error> {
        self.validate_url(&url).await?;
        let response = self.client.get(url).send().await?;
        bounded_response(response, self.budget).await
    }
}

pub async fn bounded_response(
    response: Response,
    budget: ResponseBudget,
) -> Result<Vec<u8>, Error> {
    let header_bytes = response
        .headers()
        .iter()
        .try_fold(0usize, |total, (name, value)| {
            total
                .checked_add(name.as_str().len() + value.as_bytes().len() + 4)
                .ok_or(Error::ResponseTooLarge)
        })?;
    if header_bytes > budget.max_header_bytes {
        return Err(Error::ResponseTooLarge);
    }
    if response
        .content_length()
        .is_some_and(|n| n > budget.max_body_bytes as u64)
    {
        return Err(Error::ResponseTooLarge);
    }
    let mut bytes = Vec::new();
    let mut response = response;
    while let Some(chunk) = response.chunk().await? {
        if bytes
            .len()
            .checked_add(chunk.len())
            .is_none_or(|size| size > budget.max_body_bytes)
        {
            return Err(Error::ResponseTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn validate_address(policy: NetworkPolicy, address: IpAddr) -> Result<(), Error> {
    if is_metadata(address) || address.is_unspecified() || address.is_multicast() {
        return Err(Error::ForbiddenAddress(address));
    }
    match policy {
        NetworkPolicy::PublicHttps => Ok(()),
        NetworkPolicy::LoopbackDevelopment if address.is_loopback() => Ok(()),
        NetworkPolicy::LoopbackDevelopment => Err(Error::ForbiddenAddress(address)),
        NetworkPolicy::PrivateDevice {
            allow_loopback,
            allow_link_local,
        } => {
            if address.is_loopback() && !allow_loopback {
                return Err(Error::ForbiddenAddress(address));
            }
            if is_link_local(address) && !allow_link_local {
                return Err(Error::ForbiddenAddress(address));
            }
            Ok(())
        }
    }
}
fn is_link_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v.is_link_local(),
        IpAddr::V6(v) => v.is_unicast_link_local(),
    }
}
fn is_metadata(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v.octets() == [169, 254, 169, 254],
        IpAddr::V6(v) => v.segments() == [0xfd00, 0xec2, 0, 0, 0, 0, 0, 0x254],
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTPS is required by this network policy")]
    HttpsRequired,
    #[error("URL scheme is not allowed by this network policy")]
    UnsafeScheme,
    #[error("URL contains forbidden components")]
    UnsafeUrl,
    #[error("address is forbidden by this network policy: {0}")]
    ForbiddenAddress(IpAddr),
    #[error("network policy budget is invalid")]
    InvalidBudget,
    #[error("response exceeds its configured budget")]
    ResponseTooLarge,
    #[error("DNS resolution returned no addresses")]
    ResolveEmpty,
    #[error("DNS resolution failed: {0}")]
    Resolve(std::io::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_remote_http_and_metadata() {
        assert!(
            validate_address(
                NetworkPolicy::LoopbackDevelopment,
                "127.0.0.1".parse().unwrap()
            )
            .is_ok()
        );
        assert!(
            validate_address(
                NetworkPolicy::LoopbackDevelopment,
                "192.0.2.1".parse().unwrap()
            )
            .is_err()
        );
        assert!(
            validate_address(
                NetworkPolicy::PrivateDevice {
                    allow_loopback: true,
                    allow_link_local: true
                },
                "169.254.169.254".parse().unwrap()
            )
            .is_err()
        );
    }
}
