use super::{KeyValueReportWriter, local_cpu_seed_beacon};
use crate::{
    JobScheduler, NodeStore,
    hash::hex,
    testnet::{LocalTestnet, PublicTestnetCriteria, TestnetConfig},
};

pub fn seed_local_testnet(data_dir: &str) -> std::result::Result<String, String> {
    let mut testnet = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    testnet.run_matmul_round(&scheduler);
    let matmul_settled_receipts = testnet.chain.state().settled_receipts().len();
    testnet.run_linear_training_round(&scheduler);

    let store = NodeStore::open(data_dir);
    let status = store
        .persist_chain(&testnet.chain)
        .map_err(|error| format!("failed to persist seeded local testnet chain: {error}"))?;
    let telemetry = testnet.telemetry();
    let local_evidence = testnet.public_testnet_evidence(
        &PublicTestnetCriteria {
            duration_days: 0,
            min_finality_rate_bps: 10_000,
            min_data_availability_bps: 9_500,
            ..PublicTestnetCriteria::default()
        },
        true,
    );
    let rewarded_miners = testnet
        .miners
        .iter()
        .filter(|miner| {
            testnet
                .chain
                .state()
                .pending_receipt_rewards()
                .values()
                .any(|reward| reward.beneficiary == **miner && reward.amount > 0)
        })
        .count();
    let pending_receipt_rewards = testnet.chain.state().pending_receipt_rewards().len();
    let total_reward_balance = testnet.chain.state().rewards().total_balance();
    let attestation_count: usize = testnet
        .chain
        .state()
        .attestations()
        .values()
        .map(Vec::len)
        .sum();
    let mut report = KeyValueReportWriter::new();
    report.field("command", "local_testnet_seed");
    report.field("data_dir", data_dir);
    report.field("miners", testnet.miners.len());
    report.field("validators", testnet.validators.len());
    report.field("height", testnet.chain.state().height());
    report.field("blocks", testnet.chain.blocks().len());
    report.field(
        "settled_receipts",
        testnet.chain.state().settled_receipts().len(),
    );
    report.field("matmul_settled", matmul_settled_receipts > 0);
    report.field(
        "linear_training_settled",
        !testnet.chain.state().model_states().is_empty(),
    );
    report.field("model_states", testnet.chain.state().model_states().len());
    report.field("rewarded_miners", rewarded_miners);
    report.field("pending_receipt_rewards", pending_receipt_rewards);
    report.field("total_reward_balance", total_reward_balance);
    report.field("attestation_count", attestation_count);
    report.field("total_tensor_work", telemetry.total_tensor_work);
    report.field("finality_rate_bps", local_evidence.finality_rate_bps);
    report.field(
        "data_availability_bps",
        local_evidence.data_availability_bps,
    );
    report.field("node_store_ready", true);
    report.field("persisted_block_count", status.block_count);
    report.field("latest_block_hash", hex(&status.latest_block_hash));
    report.field("public_evidence_full_spec", false);
    report.field("independently_checkable", false);
    Ok(report.finish())
}
