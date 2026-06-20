use crate::chain::{Chain, TensorBlock};
use crate::error::{Result, TvmError};
use crate::types::{Hash, hash_bytes};
use std::fs;
use std::path::{Path, PathBuf};

use super::codec::{HASH_LEN, U64_LEN, read_hash_at, read_u64_at, write_hash, write_u64};

const SNAPSHOT_MAGIC: &[u8] = b"TENSORVM_SNAPSHOT\n";
const SNAPSHOT_PAYLOAD_LEN: usize =
    U64_LEN + U64_LEN + U64_LEN + HASH_LEN + U64_LEN + HASH_LEN + HASH_LEN;
const SNAPSHOT_DIGEST_LEN: usize = HASH_LEN;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainSnapshot {
    pub height: u64,
    pub epoch: u64,
    pub finalized_beacon_round: u64,
    pub finalized_randomness: Hash,
    pub block_count: u64,
    pub state_root: Hash,
    pub latest_block_hash: Hash,
}

impl ChainSnapshot {
    pub fn from_chain(chain: &Chain) -> Self {
        Self {
            height: chain.state().height(),
            epoch: chain.state().epoch(),
            finalized_beacon_round: chain.state().finalized_beacon_round(),
            finalized_randomness: chain.state().finalized_randomness(),
            block_count: chain.blocks().len() as u64,
            state_root: chain.state_root(),
            latest_block_hash: chain
                .blocks()
                .last()
                .map(TensorBlock::hash)
                .unwrap_or([0; 32]),
        }
    }

    pub fn digest(&self) -> Hash {
        hash_bytes(b"tensor-vm-snapshot-v1", &[&self.encode_payload()])
    }

    pub fn encode(&self) -> Vec<u8> {
        let payload = self.encode_payload();
        let digest = hash_bytes(b"tensor-vm-snapshot-v1", &[&payload]);
        let mut encoded =
            Vec::with_capacity(SNAPSHOT_MAGIC.len() + SNAPSHOT_PAYLOAD_LEN + SNAPSHOT_DIGEST_LEN);
        encoded.extend_from_slice(SNAPSHOT_MAGIC);
        encoded.extend_from_slice(&payload);
        encoded.extend_from_slice(&digest);
        encoded
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if !bytes.starts_with(SNAPSHOT_MAGIC) {
            return Err(TvmError::Storage("invalid snapshot magic"));
        }

        let expected_len = SNAPSHOT_MAGIC.len() + SNAPSHOT_PAYLOAD_LEN + SNAPSHOT_DIGEST_LEN;
        if bytes.len() != expected_len {
            return Err(TvmError::Storage("invalid snapshot length"));
        }

        let payload_start = SNAPSHOT_MAGIC.len();
        let payload_end = payload_start + SNAPSHOT_PAYLOAD_LEN;
        let payload = &bytes[payload_start..payload_end];
        let expected_digest = hash_bytes(b"tensor-vm-snapshot-v1", &[payload]);
        if bytes[payload_end..] != expected_digest {
            return Err(TvmError::Storage("snapshot checksum mismatch"));
        }

        let mut offset = 0;
        Ok(Self {
            height: read_u64_at(payload, &mut offset, "truncated snapshot u64")?,
            epoch: read_u64_at(payload, &mut offset, "truncated snapshot u64")?,
            finalized_beacon_round: read_u64_at(payload, &mut offset, "truncated snapshot u64")?,
            finalized_randomness: read_hash_at(payload, &mut offset, "truncated snapshot hash")?,
            block_count: read_u64_at(payload, &mut offset, "truncated snapshot u64")?,
            state_root: read_hash_at(payload, &mut offset, "truncated snapshot hash")?,
            latest_block_hash: read_hash_at(payload, &mut offset, "truncated snapshot hash")?,
        })
    }

    fn encode_payload(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(SNAPSHOT_PAYLOAD_LEN);
        write_u64(&mut payload, self.height);
        write_u64(&mut payload, self.epoch);
        write_u64(&mut payload, self.finalized_beacon_round);
        write_hash(&mut payload, &self.finalized_randomness);
        write_u64(&mut payload, self.block_count);
        write_hash(&mut payload, &self.state_root);
        write_hash(&mut payload, &self.latest_block_hash);
        payload
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotStore {
    path: PathBuf,
}

impl SnapshotStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save(&self, snapshot: &ChainSnapshot) -> Result<()> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|_| TvmError::Storage("failed to create snapshot directory"))?;
        }

        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, snapshot.encode())
            .map_err(|_| TvmError::Storage("failed to write snapshot"))?;
        fs::rename(&temp_path, &self.path)
            .map_err(|_| TvmError::Storage("failed to commit snapshot"))?;
        Ok(())
    }

    pub fn load(&self) -> Result<ChainSnapshot> {
        let bytes =
            fs::read(&self.path).map_err(|_| TvmError::Storage("failed to read snapshot"))?;
        ChainSnapshot::decode(&bytes)
    }

    pub fn save_chain(&self, chain: &Chain) -> Result<ChainSnapshot> {
        let snapshot = ChainSnapshot::from_chain(chain);
        self.save(&snapshot)?;
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{produce_block, register_block_producer};
    use super::*;
    use crate::types::{address, hash_bytes};

    #[test]
    fn snapshot_roundtrips_through_bytes_and_detects_tampering() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"snapshot-genesis"]));
        let miner = address(b"snapshot-miner");
        register_block_producer(&mut chain, miner);
        chain.credit_account(miner, 42);
        produce_block(&mut chain, miner, 1_000);

        let snapshot = ChainSnapshot::from_chain(&chain);
        let encoded = snapshot.encode();
        assert_eq!(ChainSnapshot::decode(&encoded).unwrap(), snapshot);
        assert_eq!(
            snapshot.digest(),
            hash_bytes(b"tensor-vm-snapshot-v1", &[&snapshot.encode_payload()])
        );

        let mut tampered = encoded;
        tampered[SNAPSHOT_MAGIC.len() + 10] ^= 1;
        assert_eq!(
            ChainSnapshot::decode(&tampered),
            Err(TvmError::Storage("snapshot checksum mismatch"))
        );
    }

    #[test]
    fn snapshot_decoder_rejects_bad_magic_length_and_truncated_fields() {
        assert_eq!(
            ChainSnapshot::decode(b"bad"),
            Err(TvmError::Storage("invalid snapshot magic"))
        );

        let mut short = Vec::from(SNAPSHOT_MAGIC);
        short.extend_from_slice(&0_u64.to_le_bytes());
        assert_eq!(
            ChainSnapshot::decode(&short),
            Err(TvmError::Storage("invalid snapshot length"))
        );

        assert_eq!(
            read_u64_at(&[1, 2], &mut 0, "truncated snapshot u64"),
            Err(TvmError::Storage("truncated snapshot u64"))
        );
        assert_eq!(
            read_hash_at(&[1, 2], &mut 0, "truncated snapshot hash"),
            Err(TvmError::Storage("truncated snapshot hash"))
        );
    }

    #[test]
    fn snapshot_store_writes_and_reads_file() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"snapshot-store-genesis"]));
        let miner = address(b"snapshot-store-miner");
        register_block_producer(&mut chain, miner);
        produce_block(&mut chain, miner, 2_000);

        let path = std::env::temp_dir().join(format!(
            "tensor-vm-snapshot-{}-{}.bin",
            std::process::id(),
            chain.state().height()
        ));
        let store = SnapshotStore::new(path.clone());
        let saved = store.save_chain(&chain).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(store.path(), path.as_path());
        assert_eq!(loaded, saved);

        let _ = std::fs::remove_file(path);
    }
}
