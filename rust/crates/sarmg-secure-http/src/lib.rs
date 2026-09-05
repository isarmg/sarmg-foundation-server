//! Network policy is a closed enum; production code cannot opt into arbitrary plaintext HTTP.

use reqwest::{Client, Request, Response, redirect::Policy};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use url::{Host, Url};

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
    policy: NetworkPolicy,
    budget: ResponseBudget,
    total_timeout: Duration,
    connect_timeout: Duration,
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
        Ok(Self {
            policy,
            budget,
            total_timeout,
            connect_timeout,
        })
    }
    pub async fn validate_url(&self, url: &Url) -> Result<Vec<SocketAddr>, Error> {
        validate_url_structure(self.policy, url)?;
        let host = normalized_host(url)?;
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
        let addresses = tokio::net::lookup_host((host.as_str(), port))
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
        let addresses = self.validate_url(&url).await?;
        let request = Request::new(reqwest::Method::GET, url);
        let response = self.execute_bound(request, &addresses).await?;
        bounded_response(response, self.budget).await
    }

    /// Executes a fully constructed request while pinning the connection to the
    /// caller-supplied, already selected addresses. The request URL is validated
    /// again here so mutation between validation and execution cannot bypass the
    /// closed network policy.
    pub async fn execute_bound(
        &self,
        request: Request,
        addresses: &[SocketAddr],
    ) -> Result<Response, Error> {
        validate_url_structure(self.policy, request.url())?;
        if addresses.is_empty() {
            return Err(Error::ResolveEmpty);
        }
        for address in addresses {
            validate_address(self.policy, address.ip())?;
        }
        let host = normalized_host(request.url())?;
        let expected_port = request
            .url()
            .port_or_known_default()
            .ok_or(Error::UnsafeUrl)?;
        if addresses
            .iter()
            .any(|address| address.port() != expected_port)
        {
            return Err(Error::AddressPortMismatch);
        }
        let client = self.bound_client(&host, addresses)?;
        Ok(client.execute(request).await?)
    }

    pub async fn execute_bounded(
        &self,
        request: Request,
        addresses: &[SocketAddr],
    ) -> Result<BoundedResponse, Error> {
        let response = self.execute_bound(request, addresses).await?;
        let status = response.status();
        let body = bounded_response(response, self.budget).await?;
        Ok(BoundedResponse { status, body })
    }

    fn bound_client(&self, host: &str, addresses: &[SocketAddr]) -> Result<Client, Error> {
        let mut builder = Client::builder()
            .timeout(self.total_timeout)
            .connect_timeout(self.connect_timeout)
            .redirect(Policy::none())
            .resolve_to_addrs(host, addresses);
        if matches!(
            self.policy,
            NetworkPolicy::PrivateDevice { .. } | NetworkPolicy::LoopbackDevelopment
        ) {
            builder = builder.no_proxy();
        }
        Ok(builder.build()?)
    }
}

#[derive(Debug)]
pub struct BoundedResponse {
    pub status: reqwest::StatusCode,
    pub body: Vec<u8>,
}

pub fn validate_url_structure(policy: NetworkPolicy, url: &Url) -> Result<(), Error> {
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err(Error::UnsafeUrl);
    }
    let host = url.host().ok_or(Error::UnsafeUrl)?;
    if url.port_or_known_default().is_none() {
        return Err(Error::UnsafeUrl);
    }
    match policy {
        NetworkPolicy::PublicHttps if url.scheme() != "https" => Err(Error::HttpsRequired),
        NetworkPolicy::PrivateDevice { .. } if !matches!(url.scheme(), "http" | "https") => {
            Err(Error::UnsafeScheme)
        }
        NetworkPolicy::LoopbackDevelopment if url.scheme() != "http" => Err(Error::UnsafeScheme),
        NetworkPolicy::LoopbackDevelopment => match host {
            Host::Ipv4(address) if address.is_loopback() => Ok(()),
            Host::Ipv6(address) if address.is_loopback() => Ok(()),
            Host::Domain(domain) if domain.eq_ignore_ascii_case("localhost") => Ok(()),
            _ => Err(Error::UnsafeUrl),
        },
        _ => Ok(()),
    }
}

fn normalized_host(url: &Url) -> Result<String, Error> {
    match url.host().ok_or(Error::UnsafeUrl)? {
        Host::Domain(domain) => Ok(domain.to_owned()),
        Host::Ipv4(address) => Ok(address.to_string()),
        Host::Ipv6(address) => Ok(address.to_string()),
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
    #[error("validated address port does not match the request URL")]
    AddressPortMismatch,
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
