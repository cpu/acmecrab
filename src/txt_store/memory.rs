use crate::error::Error;
use crate::txt_store::TxtStore;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use trust_dns_server::client::rr::LowerName;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct InMemoryTxtStore {
    txt_records: HashMap<LowerName, VecDeque<String>>,
}

#[async_trait::async_trait]
impl TxtStore for InMemoryTxtStore {
    async fn add_txt(&mut self, fqdn: LowerName, value: String) -> Result<(), Error> {
        if !fqdn.is_fqdn() {
            return Err(Error::NotFQDN(fqdn));
        }
        let e = self.txt_records.entry(fqdn).or_default();
        e.insert(0, value);
        e.truncate(2);
        Ok(())
    }

    async fn get_txt(&self, fqdn: &LowerName) -> VecDeque<String> {
        self.txt_records
            .get(fqdn)
            .map_or(VecDeque::default(), Clone::clone)
    }
}
