//! Error types.

use axum::extract::rejection::JsonRejection;
use std::net::IpAddr;
use trust_dns_server::client::rr::LowerName;
use trust_dns_server::proto::error::ProtoError;

/// Error enumerates the possible ACME Crab error states.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Returned when clients `POST` the [`/register` API endpoint][crate::api#register-post].
    #[error("not implemented")]
    NotImplemented,

    /// Returned when clients `POST` the  [`/update` API endpoint][crate::api#update-post] from
    /// a source IP address that isn't in a [`Config::acl`][`crate::config::Config::acl`] network,
    /// or when the update specifies a `subdomain` that isn't mentioned in the ACL list for
    /// the client's network.
    #[error("IP {0} is not authorized to update \"{1}\"")]
    AuthForbidden(IpAddr, LowerName),

    /// Returned when clients `POST` invalid JSON.
    #[error(transparent)]
    JsonExtractorRejection(#[from] JsonRejection),

    /// Returned when clients `POST` the  [`/update` API endpoint][crate::api#update-post] with
    /// a `txt` value that isn't a valid [RFC-8555][RFC-8555] [DNS-01] challenge response value.
    ///
    /// These values MUST be a BASE64 encoded 32 byte SHA256 digest.
    ///
    /// [RFC-8555]: https://www.rfc-editor.org/rfc/rfc8555
    /// [DNS-01]: https://www.rfc-editor.org/rfc/rfc8555#section-8.4
    #[error("TXT value is not a valid DNS-01 challenge response")]
    InvalidDNS01,

    /// Returned when a non-fully qualified [`LowerName`][`trust_dns_client::rr::LowerName`] is
    /// provided to [`TxtStore::add_txt`][`crate::txt_store::TxtStore::add_txt`]
    #[error("TXT store key is not a fully qualified name: \"{0}\"")]
    NotFQDN(LowerName),

    /// Returned when the [`Config::api_bind_addr`][`crate::config::Config::api_bind_addr`] is
    /// not a loopback address, or an address within a private network space. The
    /// [ACME Crab HTTP API][crate::api] is always intended to be used on private networks
    /// that rely on network level encryption and authentication, e.g. a [Wireguard] interface.
    ///
    /// [Wireguard]: https://www.wireguard.com/#
    #[error("API bind address ({0}) must be a loopback or private IP")]
    InsecureAPIBind(IpAddr),

    /// Returned when a generic IO error occurs.
    #[error("an IO error occurred")]
    IO(#[from] std::io::Error),

    /// Returned when processing JSON from disk (e.g. to
    /// [trying to load a `Config`][crate::config::Config::try_from_file], or to
    /// [trying to load a `FileTxtStore`][crate::txt_store::file::FileTxtStore::try_from_file] fails
    /// due to invalid JSON content.
    #[error("invalid JSON")]
    InvalidJSON(#[from] serde_json::Error),

    /// Returned when the ACME Crab DNS server encounters a generic DNS protocol error.
    #[error("DNS error")]
    DNSError(#[from] ProtoError),
}
