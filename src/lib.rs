//! ACME Crab
//!
//! A very minimal stand-in for [acme-dns] well suited for [cryptokey routing] with [Wireguard].
//!
//! Self-hosted option for securely allowing ACME clients to update TXT records to solve
//! [RFC-8555][RFC-8555] [DNS-01] challenges for X509 certificate issuance. Works with all
//! authoritative DNS hosting providers that support CNAME records.
//!
//! [acme-dns]: https://github.com/joohoi/acme-dns
//! [cryptokey routing]: https://www.wireguard.com/#cryptokey-routing
//! [wireguard]: https://www.wireguard.com
//! [RFC-8555]: https://www.rfc-editor.org/rfc/rfc8555
//! [DNS-01]: https://www.rfc-editor.org/rfc/rfc8555#section-8.4
//!
#![warn(clippy::pedantic)]

pub mod api;
pub mod config;
#[doc(hidden)]
pub mod crab;
pub mod dns;
pub mod error;
pub mod txt_store;

use crate::txt_store::{file, memory};
pub use api::new as new_http;
pub use config::{Config, Shared};
pub use dns::new as new_dns;
pub use file::FileTxtStore;
pub use memory::InMemoryTxtStore;
