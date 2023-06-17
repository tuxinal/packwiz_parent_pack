use bytes::Bytes;
use digest::Digest;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Pack {
    pub index: PackIndex,
    pub options: Option<Options>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PackIndex {
    pub file: String,
    pub hash_format: HashFormat,
    pub hash: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum HashFormat {
    Sha1,
    Sha256,
    Sha512,
    Murmur2,
    Md5,
}

impl HashFormat {
    pub fn get_hash(&self, bytes: &Bytes) -> String {
        match self {
            HashFormat::Md5 => format!("{:x}", md5::Md5::new().chain_update(bytes).finalize()),
            HashFormat::Murmur2 => furse::cf_fingerprint(bytes).to_string(),
            HashFormat::Sha1 => format!("{:x}", sha1::Sha1::new().chain_update(bytes).finalize()),
            HashFormat::Sha256 => {
                format!("{:x}", sha2::Sha256::new().chain_update(bytes).finalize())
            }
            HashFormat::Sha512 => {
                format!("{:x}", sha2::Sha512::new().chain_update(bytes).finalize())
            }
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Options {
    pub parent: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Index {
    pub hash_format: HashFormat,
    pub files: Option<Vec<IndexFile>>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct IndexFile {
    pub file: String,
    pub hash: String,
    pub hash_format: Option<HashFormat>,
    pub metafile: Option<bool>,
}
