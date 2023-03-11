use crate::error::Error;
use ipnetwork::IpNetwork;
use lazy_static::lazy_static;
use serde::Deserialize;
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
use trust_dns_server::client::rr::{LowerName, Name};

pub type SharedConfig = Arc<Config>;

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub domain: LowerName,
    pub ns_domain: LowerName,
    pub ns_admin: String,
    pub txt_store_state_path: Option<String>,
    pub api_bind_addr: SocketAddr,
    #[serde_as(as = "DurationSeconds<u64>")]
    pub api_timeout: Duration,
    pub dns_udp_bind_addr: SocketAddr,
    pub dns_tcp_bind_addr: SocketAddr,
    #[serde_as(as = "DurationSeconds<u64>")]
    pub dns_tcp_timeout: Duration,
    pub acl: HashMap<IpNetwork, HashSet<LowerName>>,
    pub addrs: HashMap<LowerName, Vec<IpAddr>>,
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
    pub fn try_from_file(p: impl AsRef<Path>) -> Result<Self, Error> {
        let f = File::open(p)?;
        let reader = BufReader::new(f);
        let conf: Config = serde_json::from_reader(reader)?;
        conf.bind_addr_is_secure()?;
        Ok(conf)
    }

    pub fn update_permitted(&self, source_ip: IpAddr, subdomain: &Name) -> bool {
        self.acl
            .iter()
            .any(|(allowed_network, allowed_subdomains)| {
                allowed_network.contains(source_ip)
                    && allowed_subdomains.contains(&LowerName::from(subdomain))
            })
    }

    pub fn ns_admin(&self) -> Result<Name, Error> {
        Ok(Name::from_str(&self.sanitized_ns_admin())?)
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
