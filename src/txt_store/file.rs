use crate::error::Error;
use crate::txt_store::memory::InMemoryTxtStore;
use crate::txt_store::TxtStore;
use std::collections::VecDeque;
use std::io::ErrorKind;
use tokio::fs::File;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use trust_dns_server::client::rr::LowerName;

#[derive(Default, Debug, Clone)]
pub struct FileTxtStore {
    txt_store: InMemoryTxtStore,
    path: String,
}

impl FileTxtStore {
    pub async fn save(&self) -> Result<(), Error> {
        let data = serde_json::to_string_pretty(&self.txt_store)?;
        let mut output_file = File::create(&self.path).await?;
        output_file.write_all(data.as_bytes()).await?;
        output_file.flush().await?;
        Ok(())
    }

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

    async fn get_txt(&self, fqdn: &LowerName) -> VecDeque<String> {
        self.txt_store.get_txt(fqdn).await
    }
}
