#[cfg(test)]
use crate::chain::ChainParts;
use crate::chain::{Chain, TensorBlock};
use crate::codec;
use crate::error::{Result, TvmError};
use crate::types::{Hash, hash_bytes};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const BLOCK_LOG_MAGIC: &[u8] = b"TENSORVM_BLOCK_LOG\n";
const HASH_LEN: usize = 32;
pub(super) const BLOCK_PAYLOAD_LEN: usize = codec::TENSOR_BLOCK_PAYLOAD_LEN;
const BLOCK_DIGEST_LEN: usize = HASH_LEN;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockLogStore {
    path: PathBuf,
}

impl BlockLogStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append_block(&self, block: &TensorBlock) -> Result<()> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|_| TvmError::Storage("failed to create block log directory"))?;
        }

        let write_magic = !self.path.exists()
            || fs::metadata(&self.path)
                .map_err(|_| TvmError::Storage("failed to inspect block log"))?
                .len()
                == 0;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|_| TvmError::Storage("failed to open block log"))?;
        if write_magic {
            file.write_all(BLOCK_LOG_MAGIC)
                .map_err(|_| TvmError::Storage("failed to write block log magic"))?;
        }
        file.write_all(&encode_block_record(block))
            .map_err(|_| TvmError::Storage("failed to append block log record"))?;
        file.sync_all()
            .map_err(|_| TvmError::Storage("failed to sync block log"))?;
        Ok(())
    }

    pub fn append_chain(&self, chain: &Chain) -> Result<()> {
        for block in chain.blocks() {
            self.append_block(block)?;
        }
        Ok(())
    }

    pub fn load_blocks_or_empty(&self) -> Result<Vec<TensorBlock>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        self.load_blocks()
    }

    pub fn sync_chain(&self, chain: &Chain) -> Result<Vec<TensorBlock>> {
        let existing = self.load_blocks_or_empty()?;
        if existing.len() > chain.blocks().len() {
            return Err(TvmError::Storage("block log ahead of chain"));
        }
        for (logged, expected) in existing.iter().zip(chain.blocks()) {
            if logged != expected {
                return Err(TvmError::Storage("block log chain mismatch"));
            }
        }
        for block in chain.blocks().iter().skip(existing.len()) {
            self.append_block(block)?;
        }
        self.load_blocks_or_empty()
    }

    pub fn replace_chain(&self, chain: &Chain) -> Result<Vec<TensorBlock>> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|_| TvmError::Storage("failed to create block log directory"))?;
        }

        let temp_path = self.path.with_extension("tmp");
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|_| TvmError::Storage("failed to open replacement block log"))?;
        file.write_all(BLOCK_LOG_MAGIC)
            .map_err(|_| TvmError::Storage("failed to write block log magic"))?;
        for block in chain.blocks() {
            file.write_all(&encode_block_record(block))
                .map_err(|_| TvmError::Storage("failed to write replacement block log record"))?;
        }
        file.sync_all()
            .map_err(|_| TvmError::Storage("failed to sync replacement block log"))?;
        drop(file);
        fs::rename(&temp_path, &self.path)
            .map_err(|_| TvmError::Storage("failed to commit replacement block log"))?;
        self.load_blocks()
    }

    pub fn load_blocks(&self) -> Result<Vec<TensorBlock>> {
        let mut bytes = Vec::new();
        OpenOptions::new()
            .read(true)
            .open(&self.path)
            .map_err(|_| TvmError::Storage("failed to open block log"))?
            .read_to_end(&mut bytes)
            .map_err(|_| TvmError::Storage("failed to read block log"))?;
        decode_block_log(&bytes)
    }

    pub fn file_root(&self) -> Result<Hash> {
        let bytes = if self.path.exists() {
            fs::read(&self.path).map_err(|_| TvmError::Storage("failed to read block log"))?
        } else {
            Vec::new()
        };
        if !bytes.is_empty() {
            decode_block_log(&bytes)?;
        }
        Ok(hash_bytes(b"tensor-vm-block-log-file-root-v1", &[&bytes]))
    }
}

fn encode_block_record(block: &TensorBlock) -> Vec<u8> {
    let payload = encode_block_payload(block);
    let digest = hash_bytes(b"tensor-vm-block-log-record-v1", &[&payload]);
    let mut record = Vec::with_capacity(BLOCK_PAYLOAD_LEN + BLOCK_DIGEST_LEN);
    record.extend_from_slice(&payload);
    record.extend_from_slice(&digest);
    record
}

pub(super) fn encode_block_payload(block: &TensorBlock) -> Vec<u8> {
    codec::encode_tensor_block_payload(block)
}

fn decode_block_log(bytes: &[u8]) -> Result<Vec<TensorBlock>> {
    if !bytes.starts_with(BLOCK_LOG_MAGIC) {
        return Err(TvmError::Storage("invalid block log magic"));
    }
    let record_len = BLOCK_PAYLOAD_LEN + BLOCK_DIGEST_LEN;
    let payload = &bytes[BLOCK_LOG_MAGIC.len()..];
    if !payload.len().is_multiple_of(record_len) {
        return Err(TvmError::Storage("invalid block log length"));
    }

    let mut blocks = Vec::with_capacity(payload.len() / record_len);
    for record in payload.chunks_exact(record_len) {
        let block_payload = &record[..BLOCK_PAYLOAD_LEN];
        let expected_digest = hash_bytes(b"tensor-vm-block-log-record-v1", &[block_payload]);
        if record[BLOCK_PAYLOAD_LEN..] != expected_digest {
            return Err(TvmError::Storage("block log checksum mismatch"));
        }
        let block = decode_block_payload(block_payload)?;
        if let Some(parent) = blocks.last()
            && block.parent_hash != TensorBlock::hash(parent)
        {
            return Err(TvmError::Storage("block log parent mismatch"));
        }
        blocks.push(block);
    }
    Ok(blocks)
}

pub(super) fn decode_block_payload(bytes: &[u8]) -> Result<TensorBlock> {
    codec::decode_tensor_block_payload(bytes)
        .ok_or(TvmError::Storage("invalid block payload length"))
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{produce_block, register_block_producer};
    use super::*;
    use crate::types::{address, hash_bytes};

    fn chain_with_blocks(chain: &Chain, blocks: Vec<TensorBlock>) -> Chain {
        Chain::from_parts(ChainParts {
            params: chain.params().clone(),
            state: chain.state().clone(),
            blocks,
            block_parent_states: chain.block_parent_states.clone(),
        })
    }

    #[test]
    fn block_log_store_appends_loads_and_detects_tampering() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"block-log-genesis"]));
        let miner = address(b"block-log-miner");
        register_block_producer(&mut chain, miner);
        produce_block(&mut chain, miner, 1_000);
        produce_block(&mut chain, miner, 1_006);

        let path = std::env::temp_dir().join(format!(
            "tensor-vm-block-log-{}-{}.bin",
            std::process::id(),
            chain.state().height()
        ));
        let store = BlockLogStore::new(path.clone());
        store.append_chain(&chain).unwrap();
        let loaded = store.load_blocks().unwrap();

        assert_eq!(store.path(), path.as_path());
        assert_eq!(loaded.as_slice(), chain.blocks());
        assert_eq!(
            store.file_root().unwrap(),
            hash_bytes(
                b"tensor-vm-block-log-file-root-v1",
                &[&std::fs::read(&path).unwrap()]
            )
        );

        let mut tampered = std::fs::read(&path).unwrap();
        let last = tampered.len() - 1;
        tampered[last] ^= 1;
        std::fs::write(&path, tampered).unwrap();
        assert_eq!(
            store.load_blocks(),
            Err(TvmError::Storage("block log checksum mismatch"))
        );
        assert_eq!(
            store.file_root(),
            Err(TvmError::Storage("block log checksum mismatch"))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn block_log_sync_appends_only_missing_blocks_and_rejects_mismatches() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"block-log-sync"]));
        let miner = address(b"block-log-sync-miner");
        register_block_producer(&mut chain, miner);
        produce_block(&mut chain, miner, 1_000);
        produce_block(&mut chain, miner, 1_006);

        let path = std::env::temp_dir().join(format!(
            "tensor-vm-block-log-sync-{}-{}.bin",
            std::process::id(),
            chain.state().height()
        ));
        let store = BlockLogStore::new(path.clone());
        assert_eq!(
            store.load_blocks_or_empty().unwrap(),
            Vec::<TensorBlock>::new()
        );
        assert_eq!(
            store.file_root().unwrap(),
            hash_bytes(b"tensor-vm-block-log-file-root-v1", &[&[]])
        );

        let first = chain_with_blocks(&chain, vec![chain.blocks()[0].clone()]);
        assert_eq!(
            store.sync_chain(&first).unwrap(),
            vec![chain.blocks()[0].clone()]
        );
        assert_eq!(store.sync_chain(&chain).unwrap().as_slice(), chain.blocks());
        assert_eq!(store.sync_chain(&chain).unwrap().as_slice(), chain.blocks());
        assert_eq!(
            store.sync_chain(&first),
            Err(TvmError::Storage("block log ahead of chain"))
        );

        let mut different = Chain::new(hash_bytes(b"test", &[b"block-log-sync-other"]));
        register_block_producer(&mut different, miner);
        produce_block(&mut different, miner, 1_000);
        produce_block(&mut different, miner, 1_006);
        assert_eq!(
            store.sync_chain(&different),
            Err(TvmError::Storage("block log chain mismatch"))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn block_log_replace_chain_overwrites_ahead_log() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"block-log-replace"]));
        let miner = address(b"block-log-replace-miner");
        register_block_producer(&mut chain, miner);
        produce_block(&mut chain, miner, 1_000);
        produce_block(&mut chain, miner, 1_006);

        let path = std::env::temp_dir().join(format!(
            "tensor-vm-block-log-replace-{}-{}.bin",
            std::process::id(),
            chain.state().height()
        ));
        let store = BlockLogStore::new(path.clone());
        store.append_chain(&chain).unwrap();

        let mut shorter_blocks = chain.blocks().to_vec();
        shorter_blocks.pop();
        let shorter = chain_with_blocks(&chain, shorter_blocks);
        assert_eq!(
            store.sync_chain(&shorter),
            Err(TvmError::Storage("block log ahead of chain"))
        );

        let replaced = store.replace_chain(&shorter).unwrap();
        assert_eq!(replaced.as_slice(), shorter.blocks());
        assert_eq!(store.load_blocks().unwrap().as_slice(), shorter.blocks());

        let empty = Chain::new(hash_bytes(b"test", &[b"block-log-replace-empty"]));
        assert!(store.replace_chain(&empty).unwrap().is_empty());
        assert!(store.load_blocks().unwrap().is_empty());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn block_log_decoder_rejects_bad_magic_length_and_parent_links() {
        assert_eq!(
            decode_block_log(b"bad"),
            Err(TvmError::Storage("invalid block log magic"))
        );

        let mut truncated = Vec::from(BLOCK_LOG_MAGIC);
        truncated.push(0);
        assert_eq!(
            decode_block_log(&truncated),
            Err(TvmError::Storage("invalid block log length"))
        );
        assert_eq!(
            decode_block_payload(&[0; 4]),
            Err(TvmError::Storage("invalid block payload length"))
        );

        let mut chain = Chain::new(hash_bytes(b"test", &[b"block-log-parent"]));
        let miner = address(b"block-log-parent-miner");
        register_block_producer(&mut chain, miner);
        produce_block(&mut chain, miner, 1_000);
        let mut second = produce_block(&mut chain, miner, 1_006);
        second.parent_hash = hash_bytes(b"test", &[b"wrong-parent"]);

        let mut bytes = Vec::from(BLOCK_LOG_MAGIC);
        bytes.extend_from_slice(&encode_block_record(&chain.blocks()[0]));
        bytes.extend_from_slice(&encode_block_record(&second));
        assert_eq!(
            decode_block_log(&bytes),
            Err(TvmError::Storage("block log parent mismatch"))
        );
    }
}
