//! Configurable DNS server.
//!
//! # Dynamic TXT Records
//!
//! ACME Crab will serve a response to `TXT` class queries for subdomains of the configured
//! [`Config::domain`][`crate::config::Config::domain`], iff a [RFC-8555][RFC-8555] [DNS-01]
//! challenge response value has been provisioned by a client `POST`ing the
//! [`/update` API endpoint][crate::api#update-post].
//!
//! E.g. with config:
//! ```json
//! {
//!   "domain": "pki.example.com",
//!   "acl": { "127.0.0.1/32": [ "test" ] },
//!   ...
//! }
//! ```
//!
//! If an ACME client in the IP range `127.0.0.1` .. `127.0.0.255` `POST`s the
//! [`/update` API endpoint][crate::api#update-post] like so:
//!
//! ```bash
//! ❯ curl --json \
//!   '{"subdomain":"test","txt":"LPsIwTo7o8BoG0-vjCyGQGBWSVIPxI-i_X336eUOQZo"}' \
//!    http://localhost:3000/update
//! {"txt":"LPsIwTo7o8BoG0-vjCyGQGBWSVIPxI-i_X336eUOQZo"}   
//! ```
//!
//! Then a `TXT` class query for `test.pki.example.com` would return:
//!
//! ```bash
//! ❯ dig @127.0.0.1 -p 5353 +tcp +short test.pki.example.com TXT
//! "LPsIwTo7o8BoG0-vjCyGQGBWSVIPxI-i_X336eUOQZo"
//! ```
//!
//! [RFC-8555]: https://www.rfc-editor.org/rfc/rfc8555
//! [DNS-01]: https://www.rfc-editor.org/rfc/rfc8555#section-8.4
//!
//! # Static Records
//!
//! Several record types are served based on the static [Config][`crate::config::Config`] used
//! to create the DNS server. Unlike the dynamic TXT records, these do not change at runtime and
//! can't be influenced by the [HTTP API][crate::api].
//!
//! # A/AAAA
//!
//! ACME Crab will serve a response to `A` or `AAAA` class queries for each FQDN in the config
//! [`Config::addrs`][`crate::config::Config::addrs`] map. Only IPv4 values will be used for `A`
//! class queries, and IPv6 values for `AAAA`.
//!
//! E.g. with config:
//! ```json
//!   "addrs": {
//     "pki.example.com": ["93.184.216.34", "2606:2800:220:1:248:1893:25c8:1946" ],
//   },
//! ```
//!
//! A `A` class query for `pki.example.com` would return:
//!
//! ```bash
//! ❯ dig @127.0.0.1 -p 5353 pki.example.com +short A
//! 93.184.216.34
//! ```
//!
//! While a `AAAA` class query for `pki.example.com` would return:
//!
//! ```bash
//! ❯ dig @127.0.0.1 -p 5353 pki.example.com +short AAAA
//! 2606:2800:220:1:248:1893:25c8:1946
//! ```
//!
//! ## NS
//!
//! ACME Crab will serve a response to `NS` class queries for each FQDN in the config
//! [`Config::ns_records`][`crate::config::Config::ns_records`] map, returning each of the listed
//! [`LowerName`][`trust_dns_client::rr::LowerName`]s as authoritative answers.
//!
//! E.g. with config:
//! ```json
//! {
//!   "ns_records": {
//!     "pki.example.com": [ "ns1.pki.example.com" ]
//!   },
//!  ...
//! }
//! ```
//!
//! A `NS` class query for `pki.example.com` would return:
//! ```bash
//! ❯ dig @127.0.0.1 -p 5353 pki.example.com +short NS
//! ns1.pki.example.com.
//! ```
//!
//! ## SOA
//!
//! ACME Crab will serve a response to `SOA` class queries for
//! [`Config::domain`][`crate::config::Config::domain`]
//! using the
//! [`Config::ns_domain`][`crate::config::Config::ns_domain`] and
//! [`Config::ns_admin`][`crate::config::Config::ns_admin`] settings.
//!
//! E.g. With config:
//! ```json
//! {
//!   "domain": "pki.example.com",
//!   "ns_domain": "ns1.example.com",
//!   "ns_admin": "dns-admin@example.com",
//!   ...
//! }
//! ```
//!
//! A `SOA` class query for `pki.example.com` would return:
//! ```bash
//! ❯ dig @127.0.0.1 -p 5353 pki.example.com +short SOA
//! ns1.pki.example.com. dns-admin.example.com. 20230312 86400 7200 3600000 172800
//! ```
//!
//! _Note: The zone serial (`20230312`) will differ based on the date the query is performed._

mod handlers;
pub mod server;

pub use server::new;
