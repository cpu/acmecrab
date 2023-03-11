//! Dynamic TXT record storage.
//!
//! Supports a generic interface for setting up to two [RFC-8555][RFC-8555] [DNS-01] challenge
//! response values by FQDN.
//!
//! Two implementations are provided, [`memory::InMemoryTxtStore`] and [`file::FileTxtStore`]. The
//! former is not durable across restarts. The latter will write its state to disk for each update
//! and load this state again on startup.
//!
//! [RFC-8555]: https://www.rfc-editor.org/rfc/rfc8555
//! [DNS-01]: https://www.rfc-editor.org/rfc/rfc8555#section-8.4

use crate::error::Error;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use trust_dns_server::client::rr::LowerName;

pub mod file;
pub mod memory;

#[allow(clippy::module_name_repetitions)]
pub use file::FileTxtStore;
#[allow(clippy::module_name_repetitions)]
pub use memory::InMemoryTxtStore;

/// `DynTxtStore` is a type alias for a [`TxtStore`] that can be used by multiple read/write
/// consumers that coordinate through an [`Arc`] and a [`RwLock`] wrapping the [`TxtStore`].
#[allow(clippy::module_name_repetitions)]
pub type DynTxtStore = Arc<RwLock<dyn TxtStore + Send + Sync>>;

/// An async trait describing dynamic storage of [RFC-8555][RFC-8555] [DNS-01] challenge response
/// values, keyed by the FQDN they should be served for in the [DNS API][crate::dns].
///
/// [RFC-8555]: https://www.rfc-editor.org/rfc/rfc8555
/// [DNS-01]: https://www.rfc-editor.org/rfc/rfc8555#section-8.4
#[async_trait::async_trait]
pub trait TxtStore {
    /// Add a TXT record value for the given FQDN.
    async fn add_txt(&mut self, fqdn: LowerName, value: String) -> Result<(), Error>;

    /// Get the TXT record values for the given FQDN (if any).
    async fn get_txt(&self, fqdn: &LowerName) -> VecDeque<String>;
}
