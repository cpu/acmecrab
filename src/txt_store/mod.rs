use crate::error::Error;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use trust_dns_server::client::rr::LowerName;

pub mod file;
pub mod memory;

pub type DynTxtStore = Arc<RwLock<dyn TxtStore + Send + Sync>>;

#[async_trait::async_trait]
pub trait TxtStore {
    async fn add_txt(&mut self, fqdn: LowerName, value: String) -> Result<(), Error>;

    async fn get_txt(&self, fqdn: &LowerName) -> VecDeque<String>;
}
