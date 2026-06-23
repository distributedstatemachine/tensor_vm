use crate::chain::{Chain, ExternalRandomnessBeaconProof};
use crate::error::{Result, TvmError};
use crate::storage::NodeStore;
use crate::testnet::{
    PublicRandomnessBeaconProofKind, PublicRandomnessBeaconRecord,
    PublicRandomnessBeaconRecordStatus, PublicValidatorVrfLifecyclePhase,
    PublicValidatorVrfLifecycleRecord,
};
use crate::types::hash_bytes;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicEvidenceExportKind {
    RandomnessBeacon,
    ValidatorVrfLifecycle,
}

pub fn export_public_evidence_records(
    data_dir: &str,
    kind: PublicEvidenceExportKind,
) -> Result<String> {
    let store = NodeStore::open(data_dir);
    let chain = store.load_chain()?;
    export_public_evidence_records_from_chain(&chain, kind)
}

pub fn export_public_evidence_records_from_chain(
    chain: &Chain,
    kind: PublicEvidenceExportKind,
) -> Result<String> {
    let lines = match kind {
        PublicEvidenceExportKind::RandomnessBeacon => export_randomness_beacon_records(chain)?,
        PublicEvidenceExportKind::ValidatorVrfLifecycle => {
            export_validator_vrf_lifecycle_records(chain)?
        }
    };
    if lines.is_empty() {
        return Err(TvmError::InvalidReceipt(
            "no public evidence records to export",
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn export_randomness_beacon_records(chain: &Chain) -> Result<Vec<String>> {
    chain
        .state()
        .external_randomness_beacons()
        .values()
        .map(|record| {
            if record.beacon_round == 0 {
                return Err(TvmError::InvalidReceipt(
                    "external randomness beacon round must be nonzero",
                ));
            }
            if record.randomness == [0; 32] {
                return Err(TvmError::InvalidReceipt(
                    "external randomness root must be nonzero",
                ));
            }
            if record.proof_hash == [0; 32] {
                return Err(TvmError::InvalidReceipt(
                    "external randomness proof root must be nonzero",
                ));
            }
            let proof_kind = match record.proof {
                ExternalRandomnessBeaconProof::LocalDeterministicFixtureV1 => {
                    PublicRandomnessBeaconProofKind::LocalDeterministicFixtureV1
                }
                ExternalRandomnessBeaconProof::DrandPedersenBlsUnchainedV1 { .. }
                | ExternalRandomnessBeaconProof::DrandPedersenBlsChainedV1 { .. } => {
                    PublicRandomnessBeaconProofKind::DrandV1
                }
            };
            let source_id = hash_bytes(
                b"tensor-vm-public-randomness-source-id-v1",
                &[record.source_id.as_bytes()],
            );
            Ok(PublicRandomnessBeaconRecord {
                source_id,
                beacon_round: record.beacon_round,
                randomness_root: record.randomness,
                proof_root: record.proof_hash,
                proof_kind,
                observed_block: record.observed_at_height,
                status: PublicRandomnessBeaconRecordStatus::Accepted,
            }
            .record_line())
        })
        .collect()
}

fn export_validator_vrf_lifecycle_records(chain: &Chain) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    for reveal in chain.state().validator_vrf_reveals().values() {
        if reveal.beacon_round == 0 {
            return Err(TvmError::InvalidReceipt(
                "validator vrf beacon round must be nonzero",
            ));
        }
        if reveal.receipt_id == [0; 32] {
            return Err(TvmError::InvalidReceipt(
                "validator vrf receipt root must be nonzero",
            ));
        }
        if reveal.validator == [0; 32] {
            return Err(TvmError::InvalidReceipt(
                "validator vrf validator id must be nonzero",
            ));
        }
        if reveal.vrf_output == [0; 32] {
            return Err(TvmError::InvalidReceipt(
                "validator vrf output root must be nonzero",
            ));
        }
        if reveal.proof_hash == [0; 32] {
            return Err(TvmError::InvalidReceipt(
                "validator vrf proof root must be nonzero",
            ));
        }
        if !chain
            .state()
            .receipt_randomness_anchors()
            .contains_key(&reveal.receipt_id)
        {
            return Err(TvmError::InvalidReceipt(
                "validator vrf receipt randomness anchor missing",
            ));
        }
        lines.push(
            PublicValidatorVrfLifecycleRecord {
                receipt_root: reveal.receipt_id,
                validator_id: reveal.validator,
                beacon_round: reveal.beacon_round,
                phase: PublicValidatorVrfLifecyclePhase::Committed,
                observed_block: reveal.observed_at_height,
            }
            .record_line(),
        );
        lines.push(
            PublicValidatorVrfLifecycleRecord {
                receipt_root: reveal.receipt_id,
                validator_id: reveal.validator,
                beacon_round: reveal.beacon_round,
                phase: PublicValidatorVrfLifecyclePhase::Revealed,
                observed_block: reveal.observed_at_height,
            }
            .record_line(),
        );
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{Chain, ChainCommand, ChainEngine};
    use crate::hash::hex;
    use crate::types::hash_bytes;

    #[test]
    fn exports_chain_accepted_randomness_beacon_records() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"export-genesis"]));
        let randomness = hash_bytes(b"test", &[b"export-randomness"]);
        let proof_hash = hash_bytes(b"test", &[b"export-proof"]);
        chain
            .apply_command(ChainCommand::SubmitExternalRandomnessBeacon {
                source_id: "drand-mainnet-round-v1".to_owned(),
                beacon_round: 9,
                randomness,
                proof_hash,
            })
            .unwrap();

        let output = export_public_evidence_records_from_chain(
            &chain,
            PublicEvidenceExportKind::RandomnessBeacon,
        )
        .unwrap();

        let source_id = hash_bytes(
            b"tensor-vm-public-randomness-source-id-v1",
            &[b"drand-mainnet-round-v1"],
        );
        assert_eq!(
            output,
            format!(
                "randomness_beacon_record={},9,{},{},local-deterministic-fixture-v1,0,accepted\n",
                hex(&source_id),
                hex(&randomness),
                hex(&proof_hash)
            )
        );
    }
}
