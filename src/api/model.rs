use base64::engine::general_purpose;
use base64::{alphabet, engine, DecodeError, Engine};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone, Default, Ord, PartialOrd, Eq, PartialEq)]
pub(super) struct UpdateRecordRequest {
    pub subdomain: String,
    pub txt: String,
}

#[derive(thiserror::Error, Debug)]
pub enum TxtValidationError {
    #[error("invalid encoding: {0}")]
    InvalidEncoding(DecodeError),
    #[error("invalid decoded length: found {actual} bytes, expected {expected}")]
    InvalidDecodedLength { actual: usize, expected: usize },
}

const DNS01_DECODED_LEN_BYTES: usize = 32;

lazy_static! {
    static ref BASE64_ENGINE: engine::GeneralPurpose =
        engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD);
}

impl UpdateRecordRequest {
    pub fn valid_dns01(&self) -> Result<(), TxtValidationError> {
        match BASE64_ENGINE.decode(&self.txt) {
            Ok(raw) => match raw.len() {
                DNS01_DECODED_LEN_BYTES => Ok(()),
                _ => Err(TxtValidationError::InvalidDecodedLength {
                    actual: raw.len(),
                    expected: DNS01_DECODED_LEN_BYTES,
                }),
            },
            Err(err) => Err(TxtValidationError::InvalidEncoding(err)),
        }
    }
}

#[derive(Serialize, Debug, Clone, Default, Ord, PartialOrd, Eq, PartialEq)]
pub(super) struct UpdateRecordResult {
    pub txt: String,
}
