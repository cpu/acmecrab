use axum::extract::rejection::JsonRejection;
use std::net::IpAddr;
use trust_dns_server::client::rr::LowerName;
use trust_dns_server::proto::error::ProtoError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not implemented")]
    NotImplemented,
    #[error("IP {0} is not authorized to update \"{1}\"")]
    AuthForbidden(IpAddr, LowerName),
    #[error(transparent)]
    JsonExtractorRejection(#[from] JsonRejection),
    #[error("TXT value is not a valid DNS-01 challenge response")]
    InvalidDNS01,
    #[error("TXT store key is not a fully qualified name: \"{0}\"")]
    NotFQDN(LowerName),
    #[error("API bind address ({0}) must be a loopback or private IP")]
    InsecureAPIBind(IpAddr),
    #[error("an IO error occurred")]
    IO(#[from] std::io::Error),
    #[error("invalid JSON")]
    InvalidJSON(#[from] serde_json::Error),
    #[error("DNS error")]
    DNSError(#[from] ProtoError),
}
