//! An in-memory implementation of the [`TxtStore`][super::TxtStore] trait.
//!
//! Makes no effort to persist TXT record values between restarts.
use crate::error::Error;
use crate::txt_store::TxtStore;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use trust_dns_server::client::rr::LowerName;

/// An in-memory implementation of a dynamic TXT store. TXT values are stored in a [`HashMap`]
/// keyed by FQDN. Up to two [`String`] TXT values are maintained per FQDN using a [`VecDeque`] so
/// new values can be added to the front of the deque while old values fall off of the end.
///
/// Two TXT records per FQDN is sufficient to solve DNS-01 challenges for the base FQDN identifier
/// as well as a wildcard FQDN identifier (e.g. `foo.example.com` and `*.foo.example.com`).
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
