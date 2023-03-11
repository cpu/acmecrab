//! Configuration.

use crate::error::Error;
use crate::txt_store::DynTxtStore;
use crate::{FileTxtStore, InMemoryTxtStore};
use ipnetwork::IpNetwork;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSeconds};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use trust_dns_server::client::rr::{LowerName, Name};

/// Shared is a type alias for a reference counted [Config].
pub type Shared = Arc<Config>;

#[serde_as]
#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
/// Config describes the ACME Crab runtime configuration. All values are required unless otherwise
/// specified.
pub struct Config {
    /// Fully qualified domain name of the ACME crab instance.
    ///
    /// The [HTTP API][crate::api] allows setting dynamic TXT records for **subdomains** of this
    /// configured domain.
    pub domain: LowerName,

    /// Fully qualified domain name of the nameserver responsible for the [Config::domain].
    /// This value is used in the [SOA record][crate::dns#soa] for the [Config::domain].
    pub ns_domain: LowerName,

    /// Email address of the nameserver administrator for the [Config::domain].
    /// This value is used in the [SOA record][crate::dns#soa] for the [Config::domain].
    pub ns_admin: String,

    /// Optional path to a JSON state file to be used to persist dynamic TXT records between
    /// restarts. If omitted, an in-memory store will be used and TXT records set with the API
    /// will be lost between restarts. If provided, and the file does not exist, it will be created.
    /// If provided, and the file exists, it will be loaded to populate the initial TXT records.
    pub txt_store_state_path: Option<String>,

    /// Bind address for the [HTTP API][crate::api]. This address must be a loopback address,
    /// or an address within a private network. It must specify both an address and a port.
    pub api_bind_addr: SocketAddr,

    /// Timeout (expressed in seconds) for [HTTP API][crate::api] requests.
    #[serde_as(as = "DurationSeconds<u64>")]
    pub api_timeout: Duration,

    /// UDP bind address for responding to [DNS][crate::dns] requests.. Must specify both an
    /// address and a port.
    pub dns_udp_bind_addr: SocketAddr,

    /// TCP bind address for responding to [DNS][crate::dns] requests. Must specify both an
    /// address and a port.
    pub dns_tcp_bind_addr: SocketAddr,

    /// Timeout (expressed in seconds) for [DNS][crate::dns] requests.
    #[serde_as(as = "DurationSeconds<u64>")]
    pub dns_tcp_timeout: Duration,

    /// A mapping between [`IpNetwork`]s to a [`HashSet`] of [`LowerName`] subdomains. Clients in the
    /// `IpNetwork` key may `POST` updates for the subdomains in the associated set using the
    /// [HTTP API][crate::api] that will be served when
    /// [TXT records are queried][crate::dns#dynamic-txt-records].
    pub acl: HashMap<IpNetwork, HashSet<LowerName>>,

    /// A mapping between fully qualified [`LowerName`]s to a [`Vec`] of [`IpAddr`] values that
    /// should be served when [A/AAAA records are queried][crate::dns#aaaaa] for the keyed
    /// [`LowerName`].
    pub addrs: HashMap<LowerName, Vec<IpAddr>>,

    /// A mapping between fully qualified [`LowerName`]s to a [`Vec`] of fully qualified
    /// [`LowerName`] values that should be served when [NS records are queried][crate::dns#ns] for
    /// the keyed [`LowerName`].
    pub ns_records: HashMap<LowerName, Vec<LowerName>>,
}

lazy_static! {
    // NOTE(XXX): Once the "ip" feature has stabilized we can use Ipv6Addr.is_unique_local[0].
    //            Presently this feature is unstable so we home-roll. See also RFC 4193[1].
    // [0]: https://doc.rust-lang.org/std/net/struct.Ipv6Addr.html#method.is_unique_local
    // [1]: https://www.rfc-editor.org/rfc/rfc4193.html
    static ref IPV6_UNIQUE_LOCAL_NETWORK: IpNetwork = IpNetwork::from_str("fc00::/7").unwrap();
}

impl Config {
    /// Try to load a [Config] from the provided path.
    ///
    /// # Errors
    /// Returns [`Error::IO`] if the file at the provided path can't be opened or read.
    ///
    /// Returns [`Error::InvalidJSON`] if the file contents are not valid JSON, or the
    /// right shape to load a [Config].
    ///
    /// Returns [`Error::InsecureAPIBind`] if the API bind address in the config is not
    /// a loopback address, or an IP in a private IP range.
    pub fn try_from_file(p: impl AsRef<Path>) -> Result<Self, Error> {
        let f = File::open(p)?;
        let reader = BufReader::new(f);
        let conf: Config = serde_json::from_reader(reader)?;
        conf.bind_addr_is_secure()?;
        Ok(conf)
    }

    #[must_use]
    /// Checks if the given [`IpAddr`] is allowed to update the given [`Name`] based on the
    /// configuration ACL.
    pub fn update_permitted(&self, source_ip: IpAddr, subdomain: &Name) -> bool {
        self.acl
            .iter()
            .any(|(allowed_network, allowed_subdomains)| {
                allowed_network.contains(source_ip)
                    && allowed_subdomains.contains(&LowerName::from(subdomain))
            })
    }

    /// Returns the contact email as of the nameserver administrator as a [Name], or an error
    /// if the configured ns admin isn't a valid [Name].
    ///
    /// The '@' symbol in the contact email will be replaced by a '.'. Any '.' that appear in the
    /// user portion of the email address will be escaped to '\\.'.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DNSError`] if the configured ns admin string can't be converted to a
    /// [`Name`].
    pub fn ns_admin(&self) -> Result<Name, Error> {
        Ok(Name::from_str(&self.sanitized_ns_admin())?)
    }

    /// Return a [`DynTxtStore`] based on the configuration. If a [`Config::txt_store_state_path`] is
    /// set, a [`FileTxtStore`] is constructed using the path. Otherwise, a [`InMemoryTxtStore`] is
    /// used.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if [`FileTxtStore::try_from_file`] fails.
    pub async fn txt_store(&self) -> Result<DynTxtStore, Error> {
        match &self.txt_store_state_path {
            Some(state_path) => {
                tracing::debug!("using file-backed txt store: {state_path:?}");
                Ok(Arc::new(RwLock::new(
                    FileTxtStore::try_from_file(state_path).await?,
                )))
            }
            None => {
                tracing::debug!("using in-memory txt store");
                Ok(Arc::new(RwLock::new(InMemoryTxtStore::default())))
            }
        }
    }

    fn sanitized_ns_admin(&self) -> Cow<str> {
        match self.ns_admin.split_once('@') {
            Some((user, domain)) => {
                let user = user.replace('.', "\\.");
                Cow::Owned(format!("{user}.{domain}"))
            }
            _ => Cow::Borrowed(&self.ns_admin),
        }
    }

    fn bind_addr_is_secure(&self) -> Result<(), Error> {
        match self.api_bind_addr {
            SocketAddr::V4(v4_addr) => {
                let ip = v4_addr.ip();
                if !ip.is_loopback() && !ip.is_private() {
                    return Err(Error::InsecureAPIBind(IpAddr::V4(*ip)));
                }
                Ok(())
            }
            SocketAddr::V6(v6_addr) => {
                let ip = v6_addr.ip();
                if !ip.is_loopback() && !IPV6_UNIQUE_LOCAL_NETWORK.contains(IpAddr::V6(*ip)) {
                    return Err(Error::InsecureAPIBind(IpAddr::V6(*ip)));
                }
                Ok(())
            }
        }
    }
}
