use super::*;

#[test]
fn node_rpc_serves_head_and_blocks() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let proposer = address(b"proposer");
    chain.register_validator(proposer, 10_000).unwrap();
    chain.produce_block(proposer, 1000).unwrap();
    let rpc = RpcNode::new(chain);

    let head = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/chain/head".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(head.status, 200);
    let head = response_json(&head);
    assert_eq!(head["height"].as_u64(), Some(1));
    assert_eq!(head["block_count"].as_u64(), Some(1));
    json_hex_field(&head, "state_root");

    let health = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/health".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(health.status, 200);
    let health = response_json(&health);
    assert_eq!(health["status"].as_str(), Some("ok"));
    assert_eq!(health["service"].as_str(), Some("all"));
    assert_eq!(health["height"].as_u64(), Some(1));
    assert_eq!(health["block_count"].as_u64(), Some(1));
    assert_eq!(health["faucet_configured"].as_bool(), Some(false));

    let rpc_health = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/rpc/health".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(rpc_health.status, 200);
    let rpc_health = response_json(&rpc_health);
    assert_eq!(rpc_health["status"].as_str(), Some("ok"));
    assert_eq!(rpc_health["service"].as_str(), Some("rpc"));

    let block = rpc.handle_http_text("GET /chain/block/0 HTTP/1.1\r\n\r\n");
    assert_eq!(block.status, 200);
    let block = response_json(&block);
    assert_eq!(block["height"].as_u64(), Some(0));
    assert_eq!(block["epoch"].as_u64(), Some(0));
    json_hex_field(&block, "hash");
}

#[test]
fn node_rpc_serves_miner_and_validator_state() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"miner");
    let validator = address(b"validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    let rpc = RpcNode::new(chain);

    assert_eq!(
        rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: format!("/miners/{}", hex(&miner)),
            body: Vec::new(),
        })
        .status,
        200
    );
    assert_eq!(
        rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: format!("/validators/{}", hex(&validator)),
            body: Vec::new(),
        })
        .status,
        200
    );
}

#[test]
fn node_rpc_serves_current_jobs_and_job_lookup() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let job = MatmulJob::synthetic(0, 9, 4, 5, 6, &beacon, 20);
    let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
    let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
        model_id: hash_bytes(b"test", &[b"rpc-linear-model"]),
        step: 7,
        batch_seed: hash_bytes(b"test", &[b"rpc-linear-batch"]),
        weight_root_before: weights.commitment_root(),
        input_shape: vec![3, 2],
        weight_shape: vec![2, 2],
        target_shape: vec![3, 2],
        lr: 2,
        deadline_block: 30,
    });
    chain.submit_job(JobState::TensorOp(job.clone()));
    chain.submit_job(JobState::LinearTrainingStep(linear_job.clone()));
    let rpc = RpcNode::new(chain);

    let current = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/jobs/current".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(current.status, 200);
    let current = response_json(&current);
    let jobs = current["jobs"]
        .as_array()
        .expect("current jobs response must contain jobs array");
    assert_eq!(jobs.len(), 2);
    let current_tensor_job = jobs
        .iter()
        .find(|job| job["primitive_type"].as_str() == Some("tensor_op"))
        .expect("current jobs response must contain tensor op job");
    let current_linear_job = jobs
        .iter()
        .find(|job| job["primitive_type"].as_str() == Some("linear_training_step"))
        .expect("current jobs response must contain linear training job");
    let tensor_job_id = hex(&job.job_id);
    let linear_job_id = hex(&linear_job.job_id);
    assert_eq!(
        current_tensor_job["job_id"].as_str(),
        Some(tensor_job_id.as_str())
    );
    assert_eq!(current_tensor_job["m"].as_u64(), Some(4));
    assert_eq!(current_tensor_job["k"].as_u64(), Some(5));
    assert_eq!(current_tensor_job["n"].as_u64(), Some(6));
    assert_eq!(current_tensor_job["deadline_block"].as_u64(), Some(20));
    assert_eq!(
        current_linear_job["job_id"].as_str(),
        Some(linear_job_id.as_str())
    );
    assert_eq!(current_linear_job["step"].as_u64(), Some(7));
    assert_eq!(current_linear_job["input_shape"], serde_json::json!([3, 2]));
    assert_eq!(
        current_linear_job["weight_shape"],
        serde_json::json!([2, 2])
    );
    assert_eq!(
        current_linear_job["target_shape"],
        serde_json::json!([3, 2])
    );
    assert_eq!(current_linear_job["deadline_block"].as_u64(), Some(30));

    let response = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: format!("/jobs/{}", hex(&job.job_id)),
        body: Vec::new(),
    });
    assert_eq!(response.status, 200);
    let response = response_json(&response);
    assert_eq!(response["job_id"].as_str(), Some(tensor_job_id.as_str()));
    assert_eq!(response["primitive_type"].as_str(), Some("tensor_op"));
    assert_eq!(response["deadline_block"].as_u64(), Some(20));

    let response = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: format!("/jobs/{}", hex(&linear_job.job_id)),
        body: Vec::new(),
    });
    assert_eq!(response.status, 200);
    let response = response_json(&response);
    assert_eq!(response["job_id"].as_str(), Some(linear_job_id.as_str()));
    assert_eq!(
        response["primitive_type"].as_str(),
        Some("linear_training_step")
    );
    assert_eq!(response["step"].as_u64(), Some(7));
}

#[test]
fn node_rpc_serves_explorer_telemetry_and_faucet_routes() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"rpc-service-miner");
    let user = address(b"rpc-faucet-user");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(miner, 10_000).unwrap();
    chain.produce_block(miner, 1000).unwrap();
    let mut rpc = RpcNode::with_faucet(chain, Faucet::new(1_000, 100));

    let summary = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/explorer/summary".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(summary.status, 200);
    let summary = response_json(&summary);
    assert_eq!(summary["height"].as_u64(), Some(1));
    assert_eq!(summary["block_count"].as_u64(), Some(1));
    assert_eq!(summary["miner_count"].as_u64(), Some(1));
    assert_eq!(summary["validator_count"].as_u64(), Some(1));
    assert_eq!(summary["job_count"].as_u64(), Some(0));
    assert_eq!(summary["data_unavailable_receipt_count"].as_u64(), Some(0));
    assert_eq!(summary["data_unavailability_slash_count"].as_u64(), Some(0));
    assert_eq!(
        summary["data_unavailability_slashed_amount_total"].as_u64(),
        Some(0)
    );
    assert_eq!(
        summary["validator_audit_assignment_count"].as_u64(),
        Some(0)
    );
    assert_eq!(summary["validator_audit_result_count"].as_u64(), Some(0));
    assert_eq!(summary["validator_audit_slash_count"].as_u64(), Some(0));
    assert_eq!(
        summary["validator_audit_slashed_amount_total"].as_u64(),
        Some(0)
    );

    let overview = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/explorer/overview".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(overview.status, 200);
    let overview = response_json(&overview);
    assert_eq!(overview["type"].as_str(), Some("overview"));
    assert_eq!(overview["summary"]["miner_count"].as_u64(), Some(1));
    assert_eq!(
        overview["blocks"]
            .as_array()
            .expect("overview must include blocks")
            .len(),
        1
    );
    assert_eq!(
        overview["miners"]
            .as_array()
            .expect("overview must include miners")
            .len(),
        1
    );

    let account = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: format!("/explorer/account/{}", hex(&miner)),
        body: Vec::new(),
    });
    assert_eq!(account.status, 200);
    let account = response_json(&account);
    assert_eq!(account["type"].as_str(), Some("account"));
    assert_eq!(
        account["account"]["address"].as_str(),
        Some(hex(&miner).as_str())
    );
    assert_eq!(account["account"]["is_miner"].as_bool(), Some(true));
    assert_eq!(account["account"]["is_validator"].as_bool(), Some(true));
    assert_eq!(account["account"]["stake"].as_u64(), Some(100));

    let blocks = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/explorer/blocks/latest/1".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(blocks.status, 200);
    let blocks = response_json(&blocks);
    assert_eq!(blocks["type"].as_str(), Some("blocks"));
    let latest_blocks = blocks["blocks"]
        .as_array()
        .expect("blocks response must contain blocks array");
    assert_eq!(latest_blocks.len(), 1);
    assert_eq!(latest_blocks[0]["height"].as_u64(), Some(0));
    assert_eq!(
        latest_blocks[0]["proposer"].as_str(),
        Some(hex(&miner).as_str())
    );

    let miners = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/explorer/miners".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(miners.status, 200);
    let miners = response_json(&miners);
    assert_eq!(miners["type"].as_str(), Some("miners"));
    let miners = miners["miners"]
        .as_array()
        .expect("miners response must contain miners array");
    assert_eq!(miners.len(), 1);
    assert_eq!(miners[0]["address"].as_str(), Some(hex(&miner).as_str()));
    assert_eq!(miners[0]["hardware_class"].as_str(), Some("cpu"));
    assert_eq!(miners[0]["stake"].as_u64(), Some(100));

    let validators = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/explorer/validators".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(validators.status, 200);
    let validators = response_json(&validators);
    assert_eq!(validators["type"].as_str(), Some("validators"));
    let validators = validators["validators"]
        .as_array()
        .expect("validators response must contain validators array");
    assert_eq!(validators.len(), 1);
    assert_eq!(
        validators[0]["address"].as_str(),
        Some(hex(&miner).as_str())
    );
    assert_eq!(validators[0]["stake"].as_u64(), Some(10_000));

    let receipts = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/explorer/receipts/latest/5".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(receipts.status, 200);
    let receipts = response_json(&receipts);
    assert_eq!(receipts["type"].as_str(), Some("receipts"));
    assert!(
        receipts["receipts"]
            .as_array()
            .expect("receipts response must contain receipts array")
            .is_empty()
    );
    let bad_receipts = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/explorer/receipts/latest/nope".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(bad_receipts.status, 400);

    let jobs = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/explorer/jobs".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(jobs.status, 200);
    let jobs = response_json(&jobs);
    assert_eq!(jobs["type"].as_str(), Some("jobs"));
    assert!(
        jobs["jobs"]
            .as_array()
            .expect("jobs response must contain jobs array")
            .is_empty()
    );

    let explorer_page = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/explorer".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(explorer_page.status, 200);
    assert!(explorer_page.body.starts_with("<!doctype html>"));
    assert_eq!(
        html_tag_text(&explorer_page.body, "title"),
        "TensorVM Explorer"
    );
    assert_html_line(&explorer_page.body, "const ws = new WebSocket(WS_URL);");

    let explorer_health = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/explorer/health".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(explorer_health.status, 200);
    let explorer_health = response_json(&explorer_health);
    assert_eq!(explorer_health["status"].as_str(), Some("ok"));
    assert_eq!(explorer_health["service"].as_str(), Some("explorer"));

    let telemetry = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/telemetry".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(telemetry.status, 200);
    let telemetry = response_json(&telemetry);
    assert!(telemetry["block_finality_rate"].as_f64().is_some());
    assert_eq!(telemetry["receipt_count"].as_u64(), Some(0));
    assert_eq!(telemetry["settled_receipt_count"].as_u64(), Some(0));

    let telemetry_page = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/telemetry/dashboard".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(telemetry_page.status, 200);
    assert_eq!(
        html_tag_text(&telemetry_page.body, "title"),
        "TensorVM Telemetry"
    );
    assert_eq!(
        html_tag_text(&telemetry_page.body, "h1"),
        "Telemetry Dashboard"
    );

    let telemetry_health = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/telemetry/health".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(telemetry_health.status, 200);
    let telemetry_health = response_json(&telemetry_health);
    assert_eq!(telemetry_health["status"].as_str(), Some("ok"));
    assert_eq!(telemetry_health["service"].as_str(), Some("telemetry"));

    let faucet = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/faucet".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(faucet.status, 200);
    let faucet = response_json(&faucet);
    assert_eq!(faucet["balance"].as_u64(), Some(1_000));
    assert_eq!(faucet["drip_amount"].as_u64(), Some(100));

    let faucet_page = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/faucet/page".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(faucet_page.status, 200);
    assert_eq!(html_tag_text(&faucet_page.body, "title"), "TensorVM Faucet");
    assert_eq!(html_tag_text(&faucet_page.body, "h1"), "Faucet");
    assert_eq!(
        html_definition_value(&faucet_page.body, "Drip Amount"),
        "100"
    );

    let faucet_health = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/faucet/health".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(faucet_health.status, 200);
    let faucet_health = response_json(&faucet_health);
    assert_eq!(faucet_health["status"].as_str(), Some("ok"));
    assert_eq!(faucet_health["service"].as_str(), Some("faucet"));
    assert_eq!(faucet_health["faucet_configured"].as_bool(), Some(true));

    let claim = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: format!("/faucet/claim/{}", hex(&user)),
        body: Vec::new(),
    });
    assert_eq!(claim.status, 200);
    let claim = response_json(&claim);
    assert_eq!(claim["claimed"].as_u64(), Some(100));
    assert_eq!(claim["address"].as_str(), Some(hex(&user).as_str()));
    assert_eq!(claim["faucet_balance"].as_u64(), Some(900));
    assert_eq!(rpc.chain.state().rewards().balance(&user), 100);
    assert_eq!(rpc.faucet.as_ref().unwrap().balance(), 900);

    let duplicate = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: format!("/faucet/claim/{}", hex(&user)),
        body: Vec::new(),
    });
    assert_eq!(duplicate.status, 400);
    assert_eq!(rpc.chain.state().rewards().balance(&user), 100);
    assert_eq!(rpc.faucet.as_ref().unwrap().balance(), 900);

    let missing_faucet = RpcNode::new(Chain::new(beacon)).handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/faucet".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(missing_faucet.status, 404);
}

#[test]
fn mutable_rpc_applies_transactions_and_queues_submissions() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut rpc = RpcNode::new(Chain::new(beacon));
    let miner = address(b"rpc-miner");
    let receiver = address(b"rpc-receiver");

    let response = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/tx".to_owned(),
        body: format!("register_miner {}", hex(&miner)).into_bytes(),
    });
    assert_eq!(response.status, 202);
    assert!(rpc.chain.state().miners().contains_key(&miner));

    rpc.chain.credit_account(miner, 100);
    let response = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/tx".to_owned(),
        body: format!("transfer {} {} 70", hex(&miner), hex(&receiver)).into_bytes(),
    });
    assert_eq!(response.status, 202);
    assert_eq!(
        rpc.chain.state().accounts().get(&receiver).unwrap().balance,
        70
    );

    let tx_receipt = hash_bytes(b"test", &[b"tx-receipt"]);
    let response = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/tx".to_owned(),
        body: format!("submit_tensor_receipt {}", hex(&tx_receipt)).into_bytes(),
    });
    assert_eq!(response.status, 202);
    let duplicate = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/tx".to_owned(),
        body: format!("submit_tensor_receipt {}", hex(&tx_receipt)).into_bytes(),
    });
    assert_eq!(duplicate.status, 409);

    let linear_receipt = hash_bytes(b"test", &[b"tx-linear-receipt"]);
    let response = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/tx".to_owned(),
        body: format!("submit_linear_receipt {}", hex(&linear_receipt)).into_bytes(),
    });
    assert_eq!(response.status, 202);
    let duplicate = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/tx".to_owned(),
        body: format!("submit_linear_receipt {}", hex(&linear_receipt)).into_bytes(),
    });
    assert_eq!(duplicate.status, 409);

    let tx_attestation = hash_bytes(b"test", &[b"tx-attestation"]);
    let response = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/tx".to_owned(),
        body: format!("submit_attestation {}", hex(&tx_attestation)).into_bytes(),
    });
    assert_eq!(response.status, 202);
    let duplicate = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/tx".to_owned(),
        body: format!("submit_attestation {}", hex(&tx_attestation)).into_bytes(),
    });
    assert_eq!(duplicate.status, 202);
    assert!(rpc.chain.state().receipts().is_empty());
    assert!(rpc.chain.state().attestations().is_empty());

    let receipt = hash_bytes(b"test", &[b"receipt"]);
    let response = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/receipt".to_owned(),
        body: hex(&receipt).into_bytes(),
    });
    assert_eq!(response.status, 202);
    let duplicate = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/receipt".to_owned(),
        body: hex(&receipt).into_bytes(),
    });
    assert_eq!(duplicate.status, 409);

    let attestation = hash_bytes(b"test", &[b"attestation"]);
    let response = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/attestation".to_owned(),
        body: hex(&attestation).into_bytes(),
    });
    assert_eq!(response.status, 202);
    let duplicate = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/attestation".to_owned(),
        body: hex(&attestation).into_bytes(),
    });
    assert_eq!(duplicate.status, 202);

    let accepted_preview = rpc.handle(&RpcRequest {
        method: "POST".to_owned(),
        path: "/attestation".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(accepted_preview.status, 202);
}

#[test]
fn mutable_rpc_rejects_bad_transaction_payloads_without_mutating_state() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut rpc = RpcNode::new(Chain::new(beacon));
    let sender = address(b"rpc-sender");
    let receiver = address(b"rpc-receiver");
    let response = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/tx".to_owned(),
        body: format!("transfer {} {} 1", hex(&sender), hex(&receiver)).into_bytes(),
    });
    assert_eq!(response.status, 400);
    let malformed = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/tx".to_owned(),
        body: b"not_a_transaction".to_vec(),
    });
    assert_eq!(malformed.status, 400);
    let malformed_receipt = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/receipt".to_owned(),
        body: b"not-a-hex-receipt".to_vec(),
    });
    assert_eq!(malformed_receipt.status, 400);
    let malformed_attestation = rpc.handle_mut(&RpcRequest {
        method: "POST".to_owned(),
        path: "/attestation".to_owned(),
        body: b"not-a-hex-attestation".to_vec(),
    });
    assert_eq!(malformed_attestation.status, 400);
    assert!(rpc.txpool.is_empty());
    assert_eq!(
        rpc.chain
            .state()
            .accounts()
            .get(&receiver)
            .map(|account| account.balance),
        None
    );
}

#[test]
fn rpc_rejects_malformed_requests_and_missing_resources() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let missing = hash_bytes(b"test", &[b"missing"]);
    let mut rpc = RpcNode::new(Chain::new(beacon));

    assert_eq!(rpc.handle_http_text("").status, 400);
    assert_eq!(rpc.handle_http_text("\r\n").status, 400);
    assert_eq!(rpc.handle_http_text("GET").status, 400);
    assert_eq!(
        rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/not-a-route".to_owned(),
            body: Vec::new(),
        })
        .status,
        404
    );
    assert_eq!(
        rpc.handle(&RpcRequest {
            method: "GET".to_owned(),
            path: "/epoch/current".to_owned(),
            body: Vec::new(),
        })
        .status,
        200
    );
    let escaped_error = RpcNode::response(400, "bad \"field\"");
    assert_eq!(escaped_error.status, 400);
    assert_eq!(
        response_json(&escaped_error)["error"].as_str(),
        Some("bad \"field\"")
    );

    for (path, expected_status) in [
        ("/chain/block/nope".to_owned(), 400),
        ("/chain/block/9".to_owned(), 404),
        ("/receipts/nope".to_owned(), 400),
        (format!("/receipts/{}", hex(&missing)), 404),
        ("/explorer/account/nope".to_owned(), 400),
        ("/explorer/blocks/latest/nope".to_owned(), 400),
        ("/jobs/nope".to_owned(), 400),
        (format!("/jobs/{}", hex(&missing)), 404),
        ("/miners/nope".to_owned(), 400),
        (format!("/miners/{}", hex(&missing)), 404),
        ("/validators/nope".to_owned(), 400),
        (format!("/validators/{}", hex(&missing)), 404),
    ] {
        assert_eq!(
            rpc.handle(&RpcRequest {
                method: "GET".to_owned(),
                path,
                body: Vec::new(),
            })
            .status,
            expected_status
        );
    }

    let unconfigured_faucet_page = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: "/faucet/page".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(
        html_definition_value(&unconfigured_faucet_page.body, "Status"),
        "Not configured"
    );
    assert_eq!(
        rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/faucet/claim/nope".to_owned(),
            body: Vec::new(),
        })
        .status,
        400
    );
    assert_eq!(
        rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: format!("/faucet/claim/{}", hex(&missing)),
            body: Vec::new(),
        })
        .status,
        404
    );
    assert_eq!(
        rpc.submit_faucet_claim(&RpcRequest {
            method: "POST".to_owned(),
            path: "/wrong".to_owned(),
            body: Vec::new(),
        })
        .status,
        404
    );

    let user = address(b"exhausted-faucet-user");
    let mut exhausted = RpcNode::with_faucet(Chain::new(beacon), Faucet::new(50, 100));
    assert_eq!(
        exhausted
            .handle_mut(&RpcRequest {
                method: "POST".to_owned(),
                path: format!("/faucet/claim/{}", hex(&user)),
                body: Vec::new(),
            })
            .status,
        400
    );

    let tensor = Tensor::from_vec(vec![1, 2], DType::FieldElement, vec![1, 2]).unwrap();
    let tensor_id = rpc.insert_tensor(tensor);
    for (path, expected_status) in [
        ("/tensor/nope/descriptor".to_owned(), 404),
        (format!("/tensor/{}/descriptor", hex(&missing)), 404),
        (format!("/tensor/{}/chunk/0", hex(&missing)), 404),
        ("/tensor/nope/chunk/0".to_owned(), 404),
        (format!("/tensor/{}/chunk/nope", hex(&tensor_id)), 400),
        (format!("/tensor/{}/chunk/99", hex(&tensor_id)), 404),
        (format!("/tensor/{}/row/0", hex(&missing)), 404),
        ("/tensor/nope/row/0".to_owned(), 404),
        (format!("/tensor/{}/row/nope", hex(&tensor_id)), 400),
        (format!("/tensor/{}/row/99", hex(&tensor_id)), 404),
        (format!("/tensor/{}/opening/0", hex(&missing)), 404),
        ("/tensor/nope/opening/0".to_owned(), 404),
        (format!("/tensor/{}/opening/nope", hex(&tensor_id)), 400),
        (format!("/tensor/{}/opening/99", hex(&tensor_id)), 404),
    ] {
        assert_eq!(
            rpc.handle(&RpcRequest {
                method: "GET".to_owned(),
                path,
                body: Vec::new(),
            })
            .status,
            expected_status
        );
    }

    let receipt = hash_bytes(b"test", &[b"queued-receipt"]);
    assert_eq!(
        rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("submit_tensor_receipt {}", hex(&receipt)).into_bytes(),
        })
        .status,
        202
    );
    assert_eq!(
        rpc.handle_mut(&RpcRequest {
            method: "POST".to_owned(),
            path: "/tx".to_owned(),
            body: format!("submit_tensor_receipt {}", hex(&receipt)).into_bytes(),
        })
        .status,
        409
    );
}

#[test]
fn rpc_serves_receipts_and_status_text_edges() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"rpc-receipt-miner");
    chain.register_miner(miner, 100).unwrap();
    let job = MatmulJob::synthetic(0, 42, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = crate::jobs::TensorOpReceipt::from_job(&job, miner, 1, 5)
        .expect("static matmul receipt should build");
    let job_id = hex(&receipt.job_id);
    let receipt_id = hex(&receipt.receipt_id);
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
    let rpc = RpcNode::new(chain);

    let response = rpc.handle(&RpcRequest {
        method: "GET".to_owned(),
        path: format!("/receipts/{receipt_id}"),
        body: Vec::new(),
    });
    assert_eq!(response.status, 200);
    let response = response_json(&response);
    assert_eq!(response["receipt_id"].as_str(), Some(receipt_id.as_str()));
    assert_eq!(response["job_id"].as_str(), Some(job_id.as_str()));
    assert_eq!(response["tensor_work_units"].as_u64(), Some(16));

    for (status, text) in [
        (400, "Bad Request"),
        (401, "Unauthorized"),
        (404, "Not Found"),
        (413, "Payload Too Large"),
        (999, "Unknown"),
    ] {
        let wire = http_response_text(&RpcResponse {
            status,
            body: "{\"ok\":false}".to_owned(),
        });
        assert!(wire.starts_with(&format!("HTTP/1.1 {status} {text}")));
    }
}
