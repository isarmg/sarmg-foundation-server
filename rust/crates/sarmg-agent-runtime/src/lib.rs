//! The spool stores bounded opaque bytes and never depends on a product DTO.

use async_trait::async_trait;
use rand::Rng;
use sarmg_fs_safety::{AtomicFile, PrivateDirectory, RelativePath};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, time::Duration};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct RecordId(String);
impl RecordId {
    pub fn new() -> Result<Self, Error> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| Error::Randomness)?;
        Ok(Self(bytes.iter().map(|b| format!("{b:02x}")).collect()))
    }
    pub fn parse(value: String) -> Result<Self, Error> {
        if value.len() != 32 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::InvalidRecord);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ContractId(String);
impl ContractId {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        {
            return Err(Error::InvalidContract);
        }
        Ok(Self(value))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedBytes(Vec<u8>);
impl BoundedBytes {
    pub fn new(value: Vec<u8>, max: usize) -> Result<Self, Error> {
        if value.len() > max {
            return Err(Error::RecordTooLarge);
        }
        Ok(Self(value))
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpoolRecord {
    pub record_id: RecordId,
    pub contract_id: ContractId,
    pub created_at_micros: i64,
    pub payload: BoundedBytes,
}
pub trait AgentPayloadCodec {
    type Value;
    fn contract_id(&self) -> ContractId;
    fn encode(&self, value: &Self::Value) -> Result<BoundedBytes, Error>;
    fn validate(&self, bytes: &[u8]) -> Result<(), Error>;
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpoolLimits {
    pub max_record_bytes: usize,
    pub max_entries: usize,
    pub max_bytes: u64,
}
pub struct Spool {
    directory: PrivateDirectory,
    limits: SpoolLimits,
}
#[derive(Serialize, Deserialize)]
struct Stored {
    record_id: String,
    contract_id: String,
    created_at_micros: i64,
    payload: Vec<u8>,
}
impl Spool {
    pub fn open(path: impl AsRef<Path>, limits: SpoolLimits) -> Result<Self, Error> {
        if limits.max_record_bytes == 0 || limits.max_entries == 0 || limits.max_bytes == 0 {
            return Err(Error::InvalidLimits);
        }
        let directory = PrivateDirectory::create(path)?;
        for entry in fs::read_dir(directory.path())? {
            let path = entry?.path();
            if path.extension().is_some_and(|v| v == "tmp") {
                fs::remove_file(path)?;
            }
        }
        Ok(Self { directory, limits })
    }
    pub fn enqueue(
        &self,
        contract_id: ContractId,
        created_at_micros: i64,
        payload: BoundedBytes,
    ) -> Result<RecordId, Error> {
        if payload.0.len() > self.limits.max_record_bytes {
            return Err(Error::RecordTooLarge);
        }
        let (entries, bytes) = self.usage()?;
        if entries >= self.limits.max_entries
            || bytes
                .checked_add(payload.0.len() as u64)
                .is_none_or(|v| v > self.limits.max_bytes)
        {
            return Err(Error::SpoolFull);
        }
        let record_id = RecordId::new()?;
        let stored = Stored {
            record_id: record_id.0.clone(),
            contract_id: contract_id.0,
            created_at_micros,
            payload: payload.0,
        };
        let encoded = serde_json::to_vec(&stored).map_err(|_| Error::InvalidRecord)?;
        let name = RelativePath::new(format!(
            "{:020}-{}.record",
            created_at_micros.max(0),
            record_id.0
        ))?;
        AtomicFile::replace(&self.directory, &name, &encoded)?;
        Ok(record_id)
    }
    pub fn next(&self) -> Result<Option<SpoolRecord>, Error> {
        let mut paths = fs::read_dir(self.directory.path())?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|v| v == "record"))
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            match decode(&path, self.limits.max_record_bytes) {
                Ok(record) => return Ok(Some(record)),
                Err(_) => {
                    let quarantine = path.with_extension("bad");
                    fs::rename(&path, &quarantine)?;
                    self.directory.sync()?;
                }
            }
        }
        Ok(None)
    }
    pub fn ack(&self, id: &RecordId) -> Result<(), Error> {
        let path = self.find(id)?.ok_or(Error::RecordNotFound)?;
        fs::remove_file(path)?;
        self.directory.sync()?;
        Ok(())
    }
    pub fn usage(&self) -> Result<(usize, u64), Error> {
        let mut entries = 0;
        let mut bytes = 0u64;
        for entry in fs::read_dir(self.directory.path())? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|v| v == "record") {
                entries += 1;
                bytes = bytes.saturating_add(entry.metadata()?.len());
            }
        }
        Ok((entries, bytes))
    }
    fn find(&self, id: &RecordId) -> Result<Option<std::path::PathBuf>, Error> {
        Ok(fs::read_dir(self.directory.path())?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.ends_with(&format!("-{}.record", id.0)))
            }))
    }
}
fn decode(path: &Path, max: usize) -> Result<SpoolRecord, Error> {
    let bytes = fs::read(path)?;
    if bytes.len() > max.saturating_add(1024) {
        return Err(Error::InvalidRecord);
    }
    let stored: Stored = serde_json::from_slice(&bytes).map_err(|_| Error::InvalidRecord)?;
    Ok(SpoolRecord {
        record_id: RecordId::parse(stored.record_id)?,
        contract_id: ContractId::new(stored.contract_id)?,
        created_at_micros: stored.created_at_micros,
        payload: BoundedBytes::new(stored.payload, max)?,
    })
}
pub fn exponential_backoff(
    attempt: u32,
    base: Duration,
    maximum: Duration,
    jitter_fraction: f64,
) -> Duration {
    let factor = 1u32.checked_shl(attempt.min(30)).unwrap_or(u32::MAX);
    let bounded = base.saturating_mul(factor).min(maximum);
    let jitter = jitter_fraction.clamp(0.0, 1.0);
    let multiplier = rand::rng().random_range((1.0 - jitter)..=(1.0 + jitter));
    Duration::from_secs_f64((bounded.as_secs_f64() * multiplier).min(maximum.as_secs_f64()))
}
#[async_trait]
pub trait DeliveryTransport: Send + Sync {
    async fn deliver(&self, record: &SpoolRecord) -> Result<(), DeliveryError>;
}
#[derive(Debug, thiserror::Error)]
#[error("delivery failed: {code}")]
pub struct DeliveryError {
    pub code: String,
    pub retryable: bool,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid spool limits")]
    InvalidLimits,
    #[error("record exceeds its byte budget")]
    RecordTooLarge,
    #[error("spool capacity exhausted")]
    SpoolFull,
    #[error("invalid spool record")]
    InvalidRecord,
    #[error("invalid contract identifier")]
    InvalidContract,
    #[error("spool record not found")]
    RecordNotFound,
    #[error("secure randomness unavailable")]
    Randomness,
    #[error(transparent)]
    Filesystem(#[from] sarmg_fs_safety::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spool_is_fifo_bounded_and_acknowledged() {
        let temp = tempfile::tempdir().unwrap();
        let spool = Spool::open(
            temp.path().join("spool"),
            SpoolLimits {
                max_record_bytes: 10,
                max_entries: 2,
                max_bytes: 10000,
            },
        )
        .unwrap();
        let contract = ContractId::new("host.report").unwrap();
        let first = spool
            .enqueue(contract.clone(), 2, BoundedBytes::new(vec![2], 10).unwrap())
            .unwrap();
        spool
            .enqueue(contract, 1, BoundedBytes::new(vec![1], 10).unwrap())
            .unwrap();
        assert_eq!(spool.next().unwrap().unwrap().payload.as_slice(), &[1]);
        spool.ack(&first).unwrap();
        assert_eq!(spool.usage().unwrap().0, 1);
    }
    #[test]
    fn backoff_is_bounded() {
        for _ in 0..100 {
            assert!(
                exponential_backoff(30, Duration::from_secs(1), Duration::from_secs(60), 0.2)
                    <= Duration::from_secs(60)
            );
        }
    }
}
