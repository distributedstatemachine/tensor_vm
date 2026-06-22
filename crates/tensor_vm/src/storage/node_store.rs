use crate::chain::{Chain, ChainEngine, TensorBlock};
use crate::codec::dtype_from_tag;
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::p2p::PeerBookStore;
use crate::tensor::{Layout, Tensor};
use crate::types::Hash;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use super::codec::{StateReader, write_hash, write_i64, write_len, write_u64};
use super::{BlockLogStore, ChainSnapshot, ChainStateStore, SnapshotStore};

const SNAPSHOT_FILE_NAME: &str = "chain.snapshot";
const BLOCK_LOG_FILE_NAME: &str = "blocks.log";
const CHAIN_STATE_FILE_NAME: &str = "chain.state";
const PEER_BOOK_FILE_NAME: &str = "peers.book";
const TENSOR_ARTIFACT_DIR_NAME: &str = "tensors";
const TENSOR_ARTIFACT_EXTENSION: &str = "tensor";
const TENSOR_ARTIFACT_MAGIC: &[u8; 8] = b"TVMTEN1\0";
const MAX_TENSOR_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TENSOR_ARTIFACT_RANK: usize = 16;
const MAX_TENSOR_ARTIFACT_VALUES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedNodeState {
    pub snapshot: ChainSnapshot,
    pub blocks: Vec<TensorBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeStoreStatus {
    pub data_dir: PathBuf,
    pub snapshot: ChainSnapshot,
    pub block_count: usize,
    pub latest_block_hash: Hash,
    pub block_log_root: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TensorArtifact {
    pub tensor: Tensor,
    pub retain_until_block: u64,
}

pub trait ChainStore {
    type Chain: ChainEngine;

    fn persist_chain(&self, chain: &Self::Chain) -> Result<NodeStoreStatus>;
    fn load_chain(&self) -> Result<Self::Chain>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeStore {
    data_dir: PathBuf,
    snapshot_store: SnapshotStore,
    block_log_store: BlockLogStore,
    chain_state_store: ChainStateStore,
    peer_book_store: PeerBookStore,
}

impl NodeStore {
    pub fn open(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();
        Self {
            snapshot_store: SnapshotStore::new(data_dir.join(SNAPSHOT_FILE_NAME)),
            block_log_store: BlockLogStore::new(data_dir.join(BLOCK_LOG_FILE_NAME)),
            chain_state_store: ChainStateStore::new(data_dir.join(CHAIN_STATE_FILE_NAME)),
            peer_book_store: PeerBookStore::new(data_dir.join(PEER_BOOK_FILE_NAME)),
            data_dir,
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn snapshot_store(&self) -> &SnapshotStore {
        &self.snapshot_store
    }

    pub fn block_log_store(&self) -> &BlockLogStore {
        &self.block_log_store
    }

    pub fn chain_state_store(&self) -> &ChainStateStore {
        &self.chain_state_store
    }

    pub fn peer_book_store(&self) -> &PeerBookStore {
        &self.peer_book_store
    }

    pub fn persist_tensor(&self, tensor: &Tensor, retain_until_block: u64) -> Result<Hash> {
        let tensor_id = tensor.tensor_id();
        let artifact_dir = self.tensor_artifact_dir();
        fs::create_dir_all(&artifact_dir)
            .map_err(|_| TvmError::Storage("failed to create tensor artifact directory"))?;
        let path = self.tensor_artifact_path(&tensor_id);
        let temp_path = path.with_extension("tmp");
        let payload = encode_tensor_artifact(tensor, retain_until_block);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|_| TvmError::Storage("failed to open tensor artifact"))?;
        file.write_all(&payload)
            .map_err(|_| TvmError::Storage("failed to write tensor artifact"))?;
        file.sync_all()
            .map_err(|_| TvmError::Storage("failed to sync tensor artifact"))?;
        drop(file);
        fs::rename(&temp_path, &path)
            .map_err(|_| TvmError::Storage("failed to commit tensor artifact"))?;
        Ok(tensor_id)
    }

    pub fn load_tensors(&self) -> Result<Vec<TensorArtifact>> {
        let artifact_dir = self.tensor_artifact_dir();
        if !artifact_dir.exists() {
            return Ok(Vec::new());
        }
        let mut artifacts = Vec::new();
        let entries = fs::read_dir(&artifact_dir)
            .map_err(|_| TvmError::Storage("failed to read tensor artifact directory"))?;
        for entry in entries {
            let entry = entry
                .map_err(|_| TvmError::Storage("failed to read tensor artifact directory entry"))?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str())
                != Some(TENSOR_ARTIFACT_EXTENSION)
            {
                continue;
            }
            let metadata = entry
                .metadata()
                .map_err(|_| TvmError::Storage("failed to stat tensor artifact"))?;
            if !metadata.is_file() || metadata.len() > MAX_TENSOR_ARTIFACT_BYTES {
                return Err(TvmError::Storage("invalid tensor artifact file"));
            }
            let bytes =
                fs::read(&path).map_err(|_| TvmError::Storage("failed to read tensor artifact"))?;
            let artifact = decode_tensor_artifact(&bytes)?;
            let expected_file_name = format!(
                "{}.{}",
                hex(&artifact.tensor.tensor_id()),
                TENSOR_ARTIFACT_EXTENSION
            );
            if path.file_name().and_then(|name| name.to_str()) != Some(expected_file_name.as_str())
            {
                return Err(TvmError::Storage("tensor artifact filename mismatch"));
            }
            artifacts.push(artifact);
        }
        artifacts.sort_by_key(|artifact| artifact.tensor.tensor_id());
        Ok(artifacts)
    }

    pub fn persist_chain(&self, chain: &Chain) -> Result<NodeStoreStatus> {
        let blocks = match self.block_log_store.sync_chain(chain) {
            Ok(blocks) => blocks,
            Err(TvmError::Storage("block log chain mismatch" | "block log ahead of chain")) => {
                self.block_log_store.replace_chain(chain)?
            }
            Err(error) => return Err(error),
        };
        self.chain_state_store.save_chain(chain)?;
        let snapshot = self.snapshot_store.save_chain(chain)?;
        self.validate_parts(snapshot, blocks)
    }

    pub fn load(&self) -> Result<PersistedNodeState> {
        let snapshot = self.snapshot_store.load()?;
        let blocks = self.block_log_store.load_blocks_or_empty()?;
        match self.validate_parts(snapshot.clone(), blocks.clone()) {
            Ok(_) => Ok(PersistedNodeState { snapshot, blocks }),
            Err(error) => self.load_from_chain_state().or(Err(error)),
        }
    }

    pub fn status(&self) -> Result<NodeStoreStatus> {
        let state = self.load()?;
        self.status_from_parts(state.snapshot, &state.blocks)
    }

    pub fn compact_status(&self) -> Result<NodeStoreStatus> {
        let snapshot = self.snapshot_store.load()?;
        let blocks = self.block_log_store.load_blocks_or_empty()?;
        self.validate_parts(snapshot, blocks)
    }

    pub fn recover_from_chain_state(&self) -> Result<NodeStoreStatus> {
        let chain = self.chain_state_store.load_chain()?;
        let blocks = self.block_log_store.replace_chain(&chain)?;
        let snapshot = self.snapshot_store.save_chain(&chain)?;
        self.validate_parts(snapshot, blocks)
    }

    pub fn load_chain(&self) -> Result<Chain> {
        let mut last_error = None;
        for attempt in 0..5 {
            match self.load_chain_once() {
                Ok(chain) => return Ok(chain),
                Err(error)
                    if attempt < 4
                        && matches!(
                            error,
                            TvmError::Storage("chain state snapshot mismatch")
                                | TvmError::Storage("chain state block log mismatch")
                        ) =>
                {
                    last_error = Some(error);
                    thread::sleep(Duration::from_millis(50));
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error.unwrap_or(TvmError::Storage("chain state snapshot mismatch")))
    }

    fn load_chain_once(&self) -> Result<Chain> {
        let snapshot = self.snapshot_store.load()?;
        let blocks = self.block_log_store.load_blocks_or_empty()?;
        let chain = self.chain_state_store.load_chain()?;
        if self
            .validate_parts(snapshot.clone(), blocks.clone())
            .is_ok()
        {
            if ChainSnapshot::from_chain(&chain) != snapshot {
                return Err(TvmError::Storage("chain state snapshot mismatch"));
            }
            if chain.blocks() != blocks.as_slice() {
                return Err(TvmError::Storage("chain state block log mismatch"));
            }
        }
        Ok(chain)
    }

    fn load_from_chain_state(&self) -> Result<PersistedNodeState> {
        let chain = self.chain_state_store.load_chain()?;
        Ok(PersistedNodeState {
            snapshot: ChainSnapshot::from_chain(&chain),
            blocks: chain.blocks().to_vec(),
        })
    }

    fn validate_parts(
        &self,
        snapshot: ChainSnapshot,
        blocks: Vec<TensorBlock>,
    ) -> Result<NodeStoreStatus> {
        self.status_from_parts(snapshot, &blocks)
    }

    fn status_from_parts(
        &self,
        snapshot: ChainSnapshot,
        blocks: &[TensorBlock],
    ) -> Result<NodeStoreStatus> {
        if snapshot.block_count != blocks.len() as u64 {
            return Err(TvmError::Storage("snapshot block count mismatch"));
        }
        let latest_block_hash = blocks.last().map(TensorBlock::hash).unwrap_or([0; 32]);
        if snapshot.latest_block_hash != latest_block_hash {
            return Err(TvmError::Storage("snapshot latest block mismatch"));
        }
        Ok(NodeStoreStatus {
            data_dir: self.data_dir.clone(),
            snapshot,
            block_count: blocks.len(),
            latest_block_hash,
            block_log_root: self.block_log_store.file_root()?,
        })
    }

    fn tensor_artifact_dir(&self) -> PathBuf {
        self.data_dir.join(TENSOR_ARTIFACT_DIR_NAME)
    }

    fn tensor_artifact_path(&self, tensor_id: &Hash) -> PathBuf {
        self.tensor_artifact_dir()
            .join(format!("{}.{}", hex(tensor_id), TENSOR_ARTIFACT_EXTENSION))
    }
}

impl ChainStore for NodeStore {
    type Chain = Chain;

    fn persist_chain(&self, chain: &Self::Chain) -> Result<NodeStoreStatus> {
        NodeStore::persist_chain(self, chain)
    }

    fn load_chain(&self) -> Result<Self::Chain> {
        NodeStore::load_chain(self)
    }
}

fn encode_tensor_artifact(tensor: &Tensor, retain_until_block: u64) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(TENSOR_ARTIFACT_MAGIC);
    write_hash(&mut out, &tensor.tensor_id());
    write_hash(&mut out, &tensor.commitment_root());
    write_u64(&mut out, retain_until_block);
    write_len(&mut out, tensor.shape().len());
    for dim in tensor.shape() {
        write_u64(&mut out, *dim as u64);
    }
    out.push(tensor.dtype().tag());
    out.push(tensor.layout().tag());
    write_i64(&mut out, tensor.scale());
    write_len(&mut out, tensor.as_slice().len());
    for value in tensor.as_slice() {
        write_u64(&mut out, *value);
    }
    out
}

fn decode_tensor_artifact(bytes: &[u8]) -> Result<TensorArtifact> {
    let mut reader = StateReader::new(bytes);
    if reader.read_exact(TENSOR_ARTIFACT_MAGIC.len())? != TENSOR_ARTIFACT_MAGIC {
        return Err(TvmError::Storage("invalid tensor artifact magic"));
    }
    let expected_tensor_id = reader.read_hash()?;
    let expected_commitment_root = reader.read_hash()?;
    let retain_until_block = reader.read_u64()?;
    let rank = reader.read_len()?;
    if rank == 0 || rank > MAX_TENSOR_ARTIFACT_RANK {
        return Err(TvmError::Storage("invalid tensor artifact rank"));
    }
    let mut shape = Vec::with_capacity(rank);
    for _ in 0..rank {
        let dim = usize::try_from(reader.read_u64()?)
            .map_err(|_| TvmError::Storage("tensor artifact dimension overflow"))?;
        shape.push(dim);
    }
    let dtype = dtype_from_tag(reader.read_u8()?)
        .ok_or(TvmError::Storage("invalid tensor artifact dtype"))?;
    let layout = match reader.read_u8()? {
        1 => Layout::RowMajor,
        _ => return Err(TvmError::Storage("unsupported tensor artifact layout")),
    };
    let scale = reader.read_i64()?;
    let value_count = reader.read_len()?;
    if value_count > MAX_TENSOR_ARTIFACT_VALUES {
        return Err(TvmError::Storage("tensor artifact values too large"));
    }
    let mut values = Vec::with_capacity(value_count);
    for _ in 0..value_count {
        values.push(reader.read_u64()?);
    }
    reader.finish()?;
    let tensor = Tensor::from_vec_with_scale(shape, dtype, scale, values)?;
    if tensor.layout() != layout {
        return Err(TvmError::Storage("tensor artifact layout mismatch"));
    }
    if tensor.tensor_id() != expected_tensor_id {
        return Err(TvmError::Storage("tensor artifact id mismatch"));
    }
    if tensor.commitment_root() != expected_commitment_root {
        return Err(TvmError::Storage("tensor artifact root mismatch"));
    }
    Ok(TensorArtifact {
        tensor,
        retain_until_block,
    })
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{
        credit_reward, register_model, register_validator, submit_attestation, submit_block_vote,
        submit_job, submit_receipt, transfer,
    };
    use super::super::test_support::{produce_block, register_block_producer};
    use super::*;
    use crate::chain::{BlockVote, ChainParams, ChainParts, HardwareClass, JobState, ReceiptState};
    use crate::jobs::{
        LinearTrainingStepJob, LinearTrainingStepReceipt, LinearTrainingStepSpec, MatmulJob,
        PrimitiveType, TensorOpReceipt,
    };
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use crate::verify::{
        AttestationStatement, FreivaldsParams, ValidatorAttestation, VerificationResult,
    };

    fn chain_with_blocks(chain: &Chain, blocks: Vec<TensorBlock>) -> Chain {
        Chain::from_parts(ChainParts {
            params: chain.params().clone(),
            state: chain.state().clone(),
            blocks,
            block_parent_states: chain.block_parent_states.clone(),
            side_branch_blocks: chain.side_branch_blocks.clone(),
            side_branch_child_states: chain.side_branch_child_states.clone(),
        })
    }

    fn durable_chain_fixture(label: &[u8]) -> Chain {
        let beacon = hash_bytes(b"test", &[label]);
        let params = ChainParams {
            replication_factor: 2,
            agreement_quorum: 1,
            freivalds: FreivaldsParams {
                full_rounds: 2,
                audit_rows: 3,
                validators_per_job: 2,
                minimum_validators: 1,
                minimum_stake_numerator: 1,
                minimum_stake_denominator: 2,
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, beacon);
        let miner = address(b"durable-miner");
        let validator = address(b"durable-validator");
        chain
            .register_miner_with_profile_and_operator(
                miner,
                chain.params().miner_min_stake,
                hash_bytes(b"test", &[b"durable-operator"]),
                HardwareClass::DatacenterGpu,
                8_500,
            )
            .unwrap();
        register_validator(&mut chain, validator);
        chain.credit_account(miner, 1_000);
        transfer(&mut chain, miner, validator, 125);

        let matmul = MatmulJob::synthetic(0, 7, 4, 3, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&matmul, miner, 1, 5).unwrap();
        submit_job(&mut chain, JobState::TensorOp(matmul.clone()));
        let mut no_modulus = MatmulJob::synthetic(0, 8, 2, 2, 2, &beacon, 11);
        no_modulus.modulus = None;
        submit_job(&mut chain, JobState::TensorOp(no_modulus));
        submit_receipt(&mut chain, ReceiptState::TensorOp(receipt.clone()));
        let attestation = ValidatorAttestation::new(
            validator,
            chain.params().validator_min_stake,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"checks"]),
                data_availability_passed: true,
            },
        );
        submit_attestation(&mut chain, attestation);

        let model_id = hash_bytes(b"test", &[b"durable-model"]);
        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        register_model(
            &mut chain,
            model_id,
            hash_bytes(b"test", &[b"architecture"]),
            weights.commitment_root(),
            hash_bytes(b"test", &[b"config"]),
        );
        register_model(
            &mut chain,
            hash_bytes(b"test", &[b"durable-model-with-optimizer"]),
            hash_bytes(b"test", &[b"architecture-2"]),
            weights.commitment_root(),
            hash_bytes(b"test", &[b"config-2"]),
        );
        chain
            .set_model_optimizer_state_root_for_testing(
                model_id,
                Some(hash_bytes(b"test", &[b"optimizer"])),
            )
            .unwrap();
        let linear = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id,
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: 12,
        });
        let (linear_receipt, _output) =
            LinearTrainingStepReceipt::from_job(&linear, miner, &weights, 2, 7).unwrap();
        submit_job(&mut chain, JobState::LinearTrainingStep(linear));
        submit_receipt(
            &mut chain,
            ReceiptState::LinearTrainingStep(linear_receipt.clone()),
        );
        chain.mark_receipt_settled_for_testing(receipt.receipt_id);
        chain.mark_receipt_data_unavailable_for_testing(linear_receipt.receipt_id);
        credit_reward(&mut chain, miner, 77);
        chain.set_reward_treasury_for_testing(11);

        let block = produce_block(&mut chain, validator, 1_000);
        let validator_stake = chain.params().validator_min_stake;
        submit_block_vote(
            &mut chain,
            BlockVote::new(validator, validator_stake, &block),
        );
        produce_block(&mut chain, validator, 1_006);
        chain
    }

    #[test]
    fn node_store_persists_snapshot_block_log_and_peer_book_paths() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"node-store"]));
        let miner = address(b"node-store-miner");
        register_block_producer(&mut chain, miner);
        produce_block(&mut chain, miner, 1_000);
        produce_block(&mut chain, miner, 1_006);

        let data_dir =
            std::env::temp_dir().join(format!("tensor-vm-node-store-{}", std::process::id()));
        let store = NodeStore::open(data_dir.clone());
        assert_eq!(store.data_dir(), data_dir.as_path());
        assert_eq!(
            store.snapshot_store().path(),
            data_dir.join(SNAPSHOT_FILE_NAME).as_path()
        );
        assert_eq!(
            store.block_log_store().path(),
            data_dir.join(BLOCK_LOG_FILE_NAME).as_path()
        );
        assert_eq!(
            store.chain_state_store().path(),
            data_dir.join(CHAIN_STATE_FILE_NAME).as_path()
        );
        assert_eq!(
            store.peer_book_store().path(),
            data_dir.join(PEER_BOOK_FILE_NAME).as_path()
        );

        let status = store.persist_chain(&chain).unwrap();
        assert_eq!(status.data_dir, data_dir);
        assert_eq!(status.block_count, 2);
        assert_eq!(status.latest_block_hash, chain.blocks()[1].hash());
        assert_eq!(
            status.block_log_root,
            store.block_log_store().file_root().unwrap()
        );
        assert_eq!(status.snapshot, ChainSnapshot::from_chain(&chain));
        assert_eq!(store.status().unwrap(), status);
        let loaded = store.load().unwrap();
        assert_eq!(loaded.snapshot, status.snapshot);
        assert_eq!(loaded.blocks.as_slice(), chain.blocks());
        assert_eq!(store.load_chain().unwrap(), chain);

        produce_block(&mut chain, miner, 1_012);
        let updated = store.persist_chain(&chain).unwrap();
        assert_eq!(updated.block_count, 3);
        assert_ne!(updated.block_log_root, status.block_log_root);
        assert_eq!(
            updated.block_log_root,
            store.block_log_store().file_root().unwrap()
        );
        assert_eq!(store.load().unwrap().blocks.as_slice(), chain.blocks());
        assert_eq!(store.load_chain().unwrap(), chain);

        let _ = std::fs::remove_file(store.snapshot_store().path());
        let _ = std::fs::remove_file(store.block_log_store().path());
        let _ = std::fs::remove_file(store.chain_state_store().path());
        let _ = std::fs::remove_dir(store.data_dir());
    }

    #[test]
    fn node_store_persists_scaled_tensor_artifacts() {
        let data_dir = std::env::temp_dir().join(format!(
            "tensor-vm-node-tensor-artifact-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&data_dir);
        let store = NodeStore::open(data_dir.clone());
        let tensor =
            Tensor::from_vec_with_scale(vec![2, 2], DType::Fixed32, 4, vec![1, 2, 3, 4]).unwrap();
        let tensor_id = tensor.tensor_id();
        let root = tensor.commitment_root();

        assert_eq!(store.persist_tensor(&tensor, 77).unwrap(), tensor_id);
        let loaded = store.load_tensors().unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].retain_until_block, 77);
        assert_eq!(loaded[0].tensor, tensor);
        assert_eq!(loaded[0].tensor.scale(), 4);
        assert_eq!(loaded[0].tensor.tensor_id(), tensor_id);
        assert_eq!(loaded[0].tensor.commitment_root(), root);
        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[test]
    fn node_store_rejects_tampered_tensor_artifacts() {
        let data_dir = std::env::temp_dir().join(format!(
            "tensor-vm-node-tensor-artifact-tamper-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&data_dir);
        let store = NodeStore::open(data_dir.clone());
        let tensor = Tensor::from_vec(vec![2], DType::FieldElement, vec![9, 10]).unwrap();
        let tensor_id = tensor.tensor_id();
        store.persist_tensor(&tensor, 88).unwrap();
        let path = store.tensor_artifact_path(&tensor_id);
        let mut bytes = std::fs::read(&path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last = last.wrapping_add(1);
        std::fs::write(&path, bytes).unwrap();

        assert_eq!(
            store.load_tensors(),
            Err(TvmError::Storage("tensor artifact id mismatch"))
        );
        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[test]
    fn node_store_recovers_snapshot_and_block_log_from_chain_state() {
        let chain = durable_chain_fixture(b"node-store-recovery");
        let data_dir =
            std::env::temp_dir().join(format!("tensor-vm-node-recovery-{}", std::process::id()));
        let store = NodeStore::open(data_dir.clone());
        let original = store.persist_chain(&chain).unwrap();

        let mut stale_snapshot = ChainSnapshot::from_chain(&chain);
        stale_snapshot.block_count = stale_snapshot.block_count.saturating_sub(1);
        store.snapshot_store().save(&stale_snapshot).unwrap();
        assert_eq!(
            store.status().unwrap().snapshot,
            ChainSnapshot::from_chain(&chain)
        );

        let recovered = store.recover_from_chain_state().unwrap();
        assert_eq!(recovered.snapshot, ChainSnapshot::from_chain(&chain));
        assert_eq!(recovered.block_count, original.block_count);
        assert_eq!(store.load_chain().unwrap(), chain);

        let mut ahead = chain.clone();
        produce_block(&mut ahead, address(b"durable-validator"), 1_012);
        store
            .block_log_store()
            .append_block(ahead.blocks().last().unwrap())
            .unwrap();
        assert_eq!(store.status().unwrap().block_count, chain.blocks().len());

        let recovered_again = store.recover_from_chain_state().unwrap();
        assert_eq!(recovered_again.block_count, chain.blocks().len());
        assert_eq!(
            store.block_log_store().load_blocks().unwrap().as_slice(),
            chain.blocks()
        );
        assert_eq!(store.load_chain().unwrap(), chain);

        let _ = std::fs::remove_file(store.snapshot_store().path());
        let _ = std::fs::remove_file(store.block_log_store().path());
        let _ = std::fs::remove_file(store.chain_state_store().path());
        let _ = std::fs::remove_dir(data_dir);
    }

    #[test]
    fn node_store_satisfies_chain_store_boundary() {
        fn persist_and_reload<S>(store: &S, chain: &Chain) -> (NodeStoreStatus, Chain)
        where
            S: ChainStore<Chain = Chain>,
        {
            let status = store.persist_chain(chain).unwrap();
            let loaded = store.load_chain().unwrap();
            (status, loaded)
        }

        let mut chain = Chain::new(hash_bytes(b"test", &[b"chain-store-boundary"]));
        let miner = address(b"chain-store-boundary-miner");
        register_block_producer(&mut chain, miner);
        produce_block(&mut chain, miner, 1_000);

        let data_dir = std::env::temp_dir().join(format!(
            "tensor-vm-chain-store-boundary-{}",
            std::process::id()
        ));
        let store = NodeStore::open(data_dir);
        let (status, loaded) = persist_and_reload(&store, &chain);

        assert_eq!(status.block_count, 1);
        assert_eq!(status.latest_block_hash, chain.blocks()[0].hash());
        assert_eq!(
            status.block_log_root,
            store.block_log_store().file_root().unwrap()
        );
        assert_eq!(loaded, chain);

        let _ = std::fs::remove_file(store.snapshot_store().path());
        let _ = std::fs::remove_file(store.block_log_store().path());
        let _ = std::fs::remove_file(store.chain_state_store().path());
        let _ = std::fs::remove_dir(store.data_dir());
    }

    #[test]
    fn node_store_load_chain_rejects_state_disagreement() {
        let chain = durable_chain_fixture(b"node-store-full-state");
        let data_dir =
            std::env::temp_dir().join(format!("tensor-vm-node-full-state-{}", std::process::id()));
        let store = NodeStore::open(data_dir.clone());
        store.persist_chain(&chain).unwrap();
        assert_eq!(store.load_chain().unwrap(), chain);

        let mut changed_state = chain.clone();
        changed_state.credit_account(address(b"unexpected-account"), 1);
        store
            .chain_state_store()
            .save_chain(&changed_state)
            .unwrap();
        assert_eq!(
            store.load_chain(),
            Err(TvmError::Storage("chain state snapshot mismatch"))
        );

        let mut blocks = chain.blocks().to_vec();
        blocks[0].timestamp = blocks[0].timestamp.saturating_add(1);
        let changed_blocks = chain_with_blocks(&chain, blocks);
        store
            .chain_state_store()
            .save_chain(&changed_blocks)
            .unwrap();
        assert_eq!(
            store.load_chain(),
            Err(TvmError::Storage("chain state block log mismatch"))
        );

        let _ = std::fs::remove_file(store.snapshot_store().path());
        let _ = std::fs::remove_file(store.block_log_store().path());
        let _ = std::fs::remove_file(store.chain_state_store().path());
        let _ = std::fs::remove_dir(data_dir);
    }

    #[test]
    fn node_store_detects_snapshot_and_block_log_disagreement() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"node-store-mismatch"]));
        let miner = address(b"node-store-mismatch-miner");
        register_block_producer(&mut chain, miner);
        produce_block(&mut chain, miner, 1_000);

        let data_dir = std::env::temp_dir().join(format!(
            "tensor-vm-node-store-mismatch-{}",
            std::process::id()
        ));
        let store = NodeStore::open(data_dir.clone());
        store
            .snapshot_store()
            .save(&ChainSnapshot::from_chain(&chain))
            .unwrap();
        assert_eq!(
            store.load(),
            Err(TvmError::Storage("snapshot block count mismatch"))
        );

        store.block_log_store().append_chain(&chain).unwrap();
        let mut bad_snapshot = ChainSnapshot::from_chain(&chain);
        bad_snapshot.latest_block_hash = hash_bytes(b"test", &[b"wrong-latest"]);
        store.snapshot_store().save(&bad_snapshot).unwrap();
        assert_eq!(
            store.status(),
            Err(TvmError::Storage("snapshot latest block mismatch"))
        );

        let _ = std::fs::remove_file(store.snapshot_store().path());
        let _ = std::fs::remove_file(store.block_log_store().path());
        let _ = std::fs::remove_dir(data_dir);
    }
}
