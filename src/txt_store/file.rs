//! An JSON file-backed implementation of the [`TxtStore`][super::TxtStore] trait.
//!
//! Wraps a [`InMemoryTxtStore`][super::memory::InMemoryTxtStore] instance, persisting
//! updates to a JSON file on disk that can be reloaded across restarts.
use crate::error::Error;
use crate::txt_store::memory::InMemoryTxtStore;
use crate::txt_store::TxtStore;
use std::io::ErrorKind;
use tokio::fs::File;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use trust_dns_server::client::rr::LowerName;

/// An file-backed implementation of a dynamic TXT store. After each update a JSON file-on disk is
/// updated with the new data. This file can be reloaded across restarts to avoid losing state.
///
/// Wraps a [`InMemoryTxtStore`][super::memory::InMemoryTxtStore], operating the same way except
/// for maintaining state beyond in-memory.
#[derive(Default, Debug, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct FileTxtStore {
    txt_store: InMemoryTxtStore,
    path: String,
}

impl FileTxtStore {
    /// Save the state of the TXT store as JSON to the store's configured path, or return an Error.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidJSON`] if a record in the store can't be serialized to JSON.
    ///
    /// Returns [`Error::IO`] if the serialized TXT store state can't be written to the backing
    /// file path.
    pub async fn save(&self) -> Result<(), Error> {
        let data = serde_json::to_string_pretty(&self.txt_store)?;
        let mut output_file = File::create(&self.path).await?;
        output_file.write_all(data.as_bytes()).await?;
        output_file.flush().await?;
        Ok(())
    }

    /// Load a [`FileTxtStore`] from the JSON TXT record state located at the given path, or return
    /// an Error.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidJSON`] if the JSON state file is invalid.
    ///
    /// Returns [`Error::IO`] if the path can't be opened or read.
    pub async fn try_from_file(p: &str) -> Result<Self, Error> {
        let contents = match File::open(p).await {
            Ok(mut f) => {
                let mut buf = vec![];
                f.read_to_end(&mut buf).await?;
                buf
            }
            Err(err) => match err.kind() {
                ErrorKind::NotFound => Self::write_empty_state(File::create(&p).await?).await?,
                _ => return Err(Error::IO(err)),
            },
        };

        let txt_store: InMemoryTxtStore = serde_json::from_slice(&contents)?;
        Ok(Self {
            path: p.to_string(),
            txt_store,
        })
    }

    async fn write_empty_state(mut f: File) -> io::Result<Vec<u8>> {
        let default_data = serde_json::to_string_pretty(&InMemoryTxtStore::default())?;
        let default_bytes = default_data.as_bytes();
        f.write_all(default_bytes).await?;
        f.flush().await?;
        Ok(default_bytes.to_vec())
    }
}

#[async_trait::async_trait]
impl TxtStore for FileTxtStore {
    async fn add_txt(&mut self, fqdn: LowerName, value: String) -> Result<(), Error> {
        self.txt_store.add_txt(fqdn, value).await?;
        self.save().await?;
        Ok(())
    }

    async fn get_txt(&self, fqdn: &LowerName) -> [Option<&String>; 2] {
        self.txt_store.get_txt(fqdn).await
    }
}
