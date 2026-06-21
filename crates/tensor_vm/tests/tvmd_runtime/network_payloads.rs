use super::*;

fn test_rpc_server(chain: Chain) -> RpcHttpServer {
    let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let gateway = RpcGateway::new(node, RpcPolicy::default());
    RpcHttpServer::bind("127.0.0.1:0", gateway).unwrap()
}

fn chain_with_network_participants(
    receipt: &ReceiptState,
    attestation: &ValidatorAttestation,
) -> Chain {
    let mut chain = Chain::new(local_cpu_seed_beacon());
    register_miner(&mut chain, receipt.miner());
    register_validator(&mut chain, attestation.validator);
    chain
}

fn chain_with_network_job(
    job: tensor_vm::JobState,
    receipt: &ReceiptState,
    attestation: &ValidatorAttestation,
) -> Chain {
    let mut chain = chain_with_network_participants(receipt, attestation);
    chain.apply_command(ChainCommand::SubmitJob(job)).unwrap();
    chain
}

#[test]
fn network_ingest_fetches_pending_graph_job_program_before_retry() {
    let seed = hash_bytes(b"test", &[b"network-pending-graph-program-fetch"]);
    let source_chain = Chain::new(seed);
    let mut source = tensor_vm::SyntheticLocalJobSource::default();
    let job = tensor_vm::JobState::GraphExecution(source.next_graph_job(&source_chain));
    let job_id = job.job_id();
    let graph = tensor_vm::SyntheticLocalJobSource::graph_execution_graph();
    let program_body = graph.canonical_json().into_bytes();
    let graph_id = graph.graph_id();
    let mut server = test_rpc_server(Chain::new(seed));
    let mut pending = PendingNetworkPayloads::default();
    let provider_port = free_tcp_port();
    let provider = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{provider_port}")],
        identity_seed: Some(hash_bytes(b"test", &[b"network-graph-program-provider"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    provider.register_program(graph_id, program_body.clone());
    let requester = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
        bootstrap_addresses: vec![format!(
            "/ip4/127.0.0.1/tcp/{provider_port}/p2p/{}",
            provider.peer_id()
        )],
        identity_seed: Some(hash_bytes(b"test", &[b"network-graph-program-requester"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    wait_for_connected_role_services(&provider, &requester);
    pending.queue_job(job_id, encode_job_payload(&job));

    let ingested =
        tensor_vm::app::ingest_network_events(&mut server, &requester, false, &mut pending)
            .unwrap();

    assert_eq!(ingested.job_payloads_applied, 1);
    assert_eq!(
        server.gateway().node.chain.state().program_body(&graph_id),
        Some(program_body.as_slice())
    );
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .jobs()
            .contains_key(&job_id)
    );
    assert_eq!(pending.pending_job_count(), 0);
}

fn chain_with_network_receipt(
    job: tensor_vm::JobState,
    receipt: ReceiptState,
    attestation: &ValidatorAttestation,
) -> Chain {
    let mut chain = chain_with_network_job(job, &receipt, attestation);
    chain
        .apply_command(ChainCommand::SubmitReceipt(receipt))
        .unwrap();
    chain
}

#[test]
fn network_payload_application_defers_out_of_order_receipts_and_attestations() {
    let mut testnet = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    testnet.run_matmul_round(&scheduler);
    let job = testnet
        .chain
        .state()
        .jobs()
        .values()
        .next()
        .expect("local round must produce a job")
        .clone();
    let receipt = testnet
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("local round must produce a receipt")
        .clone();
    let receipt_id = receipt.receipt_id();
    let attestation = testnet
        .chain
        .state()
        .attestations()
        .values()
        .flat_map(|items| items.iter())
        .next()
        .expect("local round must produce an attestation")
        .clone();
    let attestation_id = attestation_announcement_hash(&attestation);

    let missing_job_chain = chain_with_network_participants(&receipt, &attestation);
    let mut missing_job_server = test_rpc_server(missing_job_chain);
    assert_eq!(
        apply_network_receipt_payload(
            &mut missing_job_server.gateway_mut().node.chain,
            receipt_id,
            &encode_receipt_payload(&receipt),
        ),
        NetworkPayloadApply::Pending
    );

    let receipt_chain = chain_with_network_job(job.clone(), &receipt, &attestation);
    let mut receipt_server = test_rpc_server(receipt_chain);
    assert_eq!(
        apply_network_receipt_payload(
            &mut receipt_server.gateway_mut().node.chain,
            receipt_id,
            &encode_receipt_payload(&receipt),
        ),
        NetworkPayloadApply::Applied
    );

    let missing_receipt_chain = chain_with_network_job(job.clone(), &receipt, &attestation);
    let mut missing_receipt_server = test_rpc_server(missing_receipt_chain);
    assert_eq!(
        apply_network_attestation_payload(
            &mut missing_receipt_server.gateway_mut().node.chain,
            attestation_id,
            &encode_attestation_payload(&attestation),
        ),
        NetworkPayloadApply::Pending
    );

    let attestation_chain = chain_with_network_receipt(job, receipt.clone(), &attestation);
    let mut attestation_server = test_rpc_server(attestation_chain);
    assert_eq!(
        apply_network_attestation_payload(
            &mut attestation_server.gateway_mut().node.chain,
            attestation_id,
            &encode_attestation_payload(&attestation),
        ),
        NetworkPayloadApply::Applied
    );
}

#[test]
fn network_applied_receipt_and_attestation_make_validator_proposal_useful() {
    let mut source = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    source.run_matmul_round(&scheduler);
    let job = source
        .chain
        .state()
        .jobs()
        .values()
        .next()
        .expect("local round must produce a job")
        .clone();
    let receipt = source
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("local round must produce a receipt")
        .clone();
    let receipt_id = receipt.receipt_id();
    let attestation = source
        .chain
        .state()
        .attestations()
        .values()
        .flat_map(|items| items.iter())
        .next()
        .expect("local round must produce an attestation")
        .clone();
    let attestation_id = attestation_announcement_hash(&attestation);
    let params = ChainParams {
        replication_factor: 1,
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            minimum_validators: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, local_cpu_seed_beacon());
    register_miner(&mut chain, receipt.miner());
    register_validator(&mut chain, attestation.validator);
    let mut server = test_rpc_server(chain);

    assert_eq!(
        apply_network_job_payload(
            &mut server.gateway_mut().node.chain,
            job.job_id(),
            &encode_job_payload(&job),
        ),
        NetworkPayloadApply::Applied
    );
    assert_eq!(
        apply_network_receipt_payload(
            &mut server.gateway_mut().node.chain,
            receipt_id,
            &encode_receipt_payload(&receipt),
        ),
        NetworkPayloadApply::Applied
    );
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .state()
            .receipts()
            .get(&receipt_id),
        Some(&receipt)
    );
    assert!(
        !server
            .gateway()
            .node
            .chain
            .state()
            .settled_receipts()
            .contains(&receipt_id)
    );

    assert_eq!(
        apply_network_attestation_payload(
            &mut server.gateway_mut().node.chain,
            attestation_id,
            &encode_attestation_payload(&attestation),
        ),
        NetworkPayloadApply::Applied
    );
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .state()
            .attestations()
            .get(&receipt_id)
            .and_then(|items| items.first()),
        Some(&attestation)
    );
    let proposal = tensor_vm::app::submit_validator_role_block_proposal(
        &mut server.gateway_mut().node,
        attestation.validator,
        1_000,
    )
    .unwrap()
    .expect("network-applied settled receipt should produce useful block");
    assert_eq!(proposal.blocks_proposed, 1);
    assert_eq!(proposal.useful_blocks_proposed, 1);
    assert_eq!(proposal.fallback_blocks_proposed, 0);
    assert_eq!(proposal.selected_receipts, vec![receipt_id]);
    let block = server.gateway().node.chain.blocks().last().unwrap();
    assert_eq!(block.proposer, attestation.validator);
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .selected_receipts_for_block(block),
        vec![receipt_id]
    );
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&block.height)
    );
}

#[test]
fn pending_network_payloads_retry_after_dependencies_arrive() {
    let mut testnet = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    testnet.run_matmul_round(&scheduler);
    let job = testnet
        .chain
        .state()
        .jobs()
        .values()
        .next()
        .expect("local round must produce a job")
        .clone();
    let job_id = job.job_id();
    let receipt = testnet
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("local round must produce a receipt")
        .clone();
    let receipt_id = receipt.receipt_id();
    let attestation = testnet
        .chain
        .state()
        .attestations()
        .values()
        .flat_map(|items| items.iter())
        .next()
        .expect("local round must produce an attestation")
        .clone();
    let attestation_id = attestation_announcement_hash(&attestation);

    let out_of_order_chain = chain_with_network_participants(&receipt, &attestation);
    let mut server = test_rpc_server(out_of_order_chain);
    let mut pending = PendingNetworkPayloads::default();

    assert_eq!(
        apply_network_receipt_payload(
            &mut server.gateway_mut().node.chain,
            receipt_id,
            &encode_receipt_payload(&receipt)
        ),
        NetworkPayloadApply::Pending
    );
    pending.queue_receipt(receipt_id, encode_receipt_payload(&receipt));
    assert_eq!(
        apply_network_attestation_payload(
            &mut server.gateway_mut().node.chain,
            attestation_id,
            &encode_attestation_payload(&attestation),
        ),
        NetworkPayloadApply::Pending
    );
    pending.queue_attestation(attestation_id, encode_attestation_payload(&attestation));

    assert_eq!(
        apply_network_job_payload(
            &mut server.gateway_mut().node.chain,
            job_id,
            &encode_job_payload(&job),
        ),
        NetworkPayloadApply::Applied
    );
    let mut processor = ChainNetworkPayloadProcessor::new(&mut server.gateway_mut().node.chain);
    let retried = pending.retry_with(&mut processor);

    assert!(retried.has_activity());
    assert_eq!(retried.receipt_payloads_applied, 1);
    assert_eq!(retried.attestation_payloads_applied, 1);
    assert_eq!(retried.invalid_events, 0);
    assert!(pending.is_empty());
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .state()
            .receipts()
            .get(&receipt_id),
        Some(&receipt)
    );
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .state()
            .attestations()
            .get(&receipt_id)
            .and_then(|items| items.first()),
        Some(&attestation)
    );
}

#[test]
fn network_ingest_orders_payload_dependencies_before_blocks() {
    let block_hash = hash_bytes(b"test", &[b"announced-block"]);
    let job_id = hash_bytes(b"test", &[b"announced-job"]);
    let receipt_id = hash_bytes(b"test", &[b"announced-receipt"]);
    let messages = network_ingest_order(vec![
        P2pMessage::NewJobPayload {
            job_id,
            payload: vec![1, 2, 3],
        },
        P2pMessage::NewReceipt(receipt_id),
        P2pMessage::NewBlockHeader {
            height: 3,
            block_hash,
        },
        P2pMessage::NewBlockPayload {
            height: 3,
            block_hash,
            payload: vec![4, 5, 6],
        },
        P2pMessage::NewJob(job_id),
        P2pMessage::NewBlock(block_hash),
    ]);

    assert!(matches!(messages[0], P2pMessage::NewJobPayload { .. }));
    assert!(matches!(messages[1], P2pMessage::NewReceipt(_)));
    assert!(matches!(messages[2], P2pMessage::NewJob(_)));
    assert!(matches!(messages[3], P2pMessage::NewBlockPayload { .. }));
    assert!(matches!(messages[4], P2pMessage::NewBlockHeader { .. }));
    assert!(matches!(messages[5], P2pMessage::NewBlock(_)));
}
