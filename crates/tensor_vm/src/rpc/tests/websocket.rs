use super::*;

#[test]
fn explorer_websocket_views_cover_chain_collections_and_bad_commands() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let cpu_miner = address(b"ws-cpu-miner");
    let consumer_gpu_miner = address(b"ws-consumer-gpu-miner");
    let datacenter_gpu_miner = address(b"ws-datacenter-gpu-miner");
    let other_miner = address(b"ws-other-miner");
    let validator = address(b"ws-validator");
    chain.register_miner(cpu_miner, 100).unwrap();
    chain
        .register_miner_with_profile(consumer_gpu_miner, 100, HardwareClass::ConsumerGpu, 9_000)
        .unwrap();
    chain
        .register_miner_with_profile(
            datacenter_gpu_miner,
            100,
            HardwareClass::DatacenterGpu,
            8_000,
        )
        .unwrap();
    chain
        .register_miner_with_profile(other_miner, 100, HardwareClass::Other, 0)
        .unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    let matmul_job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&matmul_job, cpu_miner, 1, 5).unwrap();
    let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
    let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
        model_id: hash_bytes(b"test", &[b"ws-linear-model"]),
        step: 3,
        batch_seed: hash_bytes(b"test", &[b"ws-linear-batch"]),
        weight_root_before: weights.commitment_root(),
        input_shape: vec![3, 2],
        weight_shape: vec![2, 2],
        target_shape: vec![3, 2],
        lr: 1,
        deadline_block: 20,
    });
    let graph = SyntheticLocalJobSource::graph_execution_graph();
    let graph_id = graph.validate_for_consensus().unwrap();
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    let mut job_source = SyntheticLocalJobSource::new(JobScheduler::default());
    let graph_job = job_source.next_graph_job(&chain);
    let graph_inputs = SyntheticLocalJobSource::graph_execution_inputs();
    let (graph_receipt, _graph_outputs) =
        GraphReceipt::from_execution(&graph_job, &graph, cpu_miner, &graph_inputs, 2, 6).unwrap();
    chain.submit_job(JobState::TensorOp(matmul_job));
    chain.submit_job(JobState::LinearTrainingStep(linear_job));
    chain
        .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(
            graph_job.clone(),
        )))
        .unwrap();
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
    chain
        .apply_command(ChainCommand::SubmitReceipt(ReceiptState::GraphExecution(
            graph_receipt.clone(),
        )))
        .unwrap();
    chain.mark_receipt_settled_for_testing(receipt.receipt_id);
    chain.register_validator(cpu_miner, 10_000).unwrap();
    chain.produce_block(cpu_miner, 1000).unwrap();
    let rpc = RpcNode::new(chain);

    let miners = rpc.explorer_websocket_response("miners");
    let miners = json_text(&miners);
    assert_eq!(miners["type"].as_str(), Some("miners"));
    let mut hardware_classes = miners["miners"]
        .as_array()
        .expect("miners response must contain miners array")
        .iter()
        .map(|miner| {
            miner["hardware_class"]
                .as_str()
                .expect("miner hardware class must be a string")
        })
        .collect::<Vec<_>>();
    hardware_classes.sort_unstable();
    assert_eq!(
        hardware_classes,
        ["consumer_gpu", "cpu", "datacenter_gpu", "other"]
    );

    let validators = rpc.explorer_websocket_response("{\"type\":\"validators\"}");
    let validators = json_text(&validators);
    assert_eq!(validators["type"].as_str(), Some("validators"));
    let validators = validators["validators"]
        .as_array()
        .expect("validators response must contain validators array");
    assert_eq!(validators.len(), 2);
    assert!(
        validators
            .iter()
            .all(|validator| validator["valid_attestations"].as_u64().is_some())
    );

    let jobs = rpc.explorer_websocket_response("{\"type\":\"jobs\",\"job_limit\":3}");
    let jobs = json_text(&jobs);
    assert_eq!(jobs["type"].as_str(), Some("jobs"));
    let mut primitive_types = jobs["jobs"]
        .as_array()
        .expect("jobs response must contain jobs array")
        .iter()
        .map(|job| {
            job["primitive_type"]
                .as_str()
                .expect("job primitive type must be a string")
        })
        .collect::<Vec<_>>();
    primitive_types.sort_unstable();
    assert_eq!(
        primitive_types,
        ["graph_execution", "linear_training_step", "tensor_op"]
    );

    let receipts = rpc.explorer_websocket_response("{\"type\":\"receipts\",\"receipt_limit\":2}");
    let receipts = json_text(&receipts);
    assert_eq!(receipts["type"].as_str(), Some("receipts"));
    let receipts = receipts["receipts"]
        .as_array()
        .expect("receipts response must contain receipts array");
    assert_eq!(receipts.len(), 2);
    let mut receipt_primitive_types = receipts
        .iter()
        .map(|receipt| {
            receipt["primitive_type"]
                .as_str()
                .expect("receipt primitive type must be a string")
        })
        .collect::<Vec<_>>();
    receipt_primitive_types.sort_unstable();
    assert_eq!(receipt_primitive_types, ["graph_execution", "tensor_op"]);
    let tensor_op_receipt = receipts
        .iter()
        .find(|receipt| receipt["primitive_type"].as_str() == Some("tensor_op"))
        .expect("receipts response must include the tensor op receipt");
    assert_eq!(tensor_op_receipt["attestation_count"].as_u64(), Some(0));
    assert!(
        tensor_op_receipt["validator_attestations"]
            .as_array()
            .expect("receipt validator attestations must be an array")
            .is_empty()
    );
    assert_eq!(tensor_op_receipt["settled"].as_bool(), Some(true));

    let blocks = rpc.explorer_websocket_response("{\"type\":\"blocks\",\"block_limit\":1}");
    let blocks = json_text(&blocks);
    assert_eq!(blocks["type"].as_str(), Some("blocks"));
    let blocks = blocks["blocks"]
        .as_array()
        .expect("blocks response must contain blocks array");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0]["height"].as_u64(), Some(0));

    let summary = rpc.explorer_websocket_response("summary");
    let summary = json_text(&summary);
    assert_eq!(summary["type"].as_str(), Some("summary"));
    assert_eq!(summary["summary"]["miner_count"].as_u64(), Some(4));
    assert_eq!(summary["summary"]["job_count"].as_u64(), Some(3));
    let spaced_summary = rpc.explorer_websocket_response("{\"type\": \"summary\"}");
    let spaced_summary = json_text(&spaced_summary);
    assert_eq!(spaced_summary["type"].as_str(), Some("summary"));

    let unknown_type = rpc.explorer_websocket_response("{\"type\":\"other\",\"block_limit\":1}");
    let unknown_type = json_text(&unknown_type);
    assert_eq!(unknown_type["type"].as_str(), Some("overview"));
    assert_eq!(
        unknown_type["blocks"]
            .as_array()
            .expect("overview must include blocks")
            .len(),
        1
    );

    let missing_account = rpc.explorer_websocket_response("{\"type\":\"account\"}");
    let missing_account = json_text(&missing_account);
    assert_eq!(missing_account["type"].as_str(), Some("error"));
    assert_eq!(
        missing_account["error"].as_str(),
        Some("missing account address")
    );
    let invalid_account =
        rpc.explorer_websocket_response("{\"type\":\"account\",\"address\":\"bad\"}");
    let invalid_account = json_text(&invalid_account);
    assert_eq!(invalid_account["type"].as_str(), Some("error"));
    assert_eq!(
        invalid_account["error"].as_str(),
        Some("invalid account address")
    );

    assert_eq!(primitive_label(PrimitiveType::TensorOp), "tensor_op");
    assert_eq!(
        primitive_label(PrimitiveType::LinearTrainingStep),
        "linear_training_step"
    );
    assert_eq!(hardware_class_label(HardwareClass::Cpu), "cpu");
    assert_eq!(
        hardware_class_label(HardwareClass::ConsumerGpu),
        "consumer_gpu"
    );
    assert_eq!(
        hardware_class_label(HardwareClass::DatacenterGpu),
        "datacenter_gpu"
    );
    assert_eq!(hardware_class_label(HardwareClass::Other), "other");
}

#[test]
fn websocket_frame_helpers_cover_close_errors_and_extended_lengths() {
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpListener, TcpStream};

    assert_eq!(
        read_single_websocket_frame(&[0x81, 126, 0, 126])
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::UnexpectedEof
    );
    let mut extended_16 = vec![0x81, 126];
    extended_16.extend_from_slice(&(126_u16).to_be_bytes());
    extended_16.extend(std::iter::repeat_n(b'a', 126));
    assert_eq!(
        read_single_websocket_frame(&extended_16).unwrap(),
        Some("a".repeat(126))
    );
    let mut extended_64 = vec![0x81, 127];
    extended_64.extend_from_slice(&(3_u64).to_be_bytes());
    extended_64.extend_from_slice(b"hey");
    assert_eq!(
        read_single_websocket_frame(&extended_64).unwrap(),
        Some("hey".to_owned())
    );
    let mut too_large = vec![0x81, 127];
    too_large.extend_from_slice(&((64_u64 * 1024) + 1).to_be_bytes());
    assert_eq!(
        read_single_websocket_frame(&too_large).unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
    assert_eq!(
        read_single_websocket_frame(&[0x81, 1, 0xff])
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidData
    );
    assert_eq!(
        read_single_websocket_frame(&[0x82, 0]).unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
    assert_eq!(read_single_websocket_frame(&[0x88, 0]).unwrap(), None);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let writer = std::thread::spawn(move || {
        let (mut server, _) = listener.accept().unwrap();
        let small_payload = [b'a'; 126];
        let large_payload = vec![b'b'; 65_536];
        write_websocket_frame(&mut server, 0x1, &small_payload).unwrap();
        write_websocket_frame(&mut server, 0x1, &large_payload).unwrap();
    });
    let mut client = TcpStream::connect(addr).unwrap();
    let mut raw = Vec::new();
    client.read_to_end(&mut raw).unwrap();
    writer.join().unwrap();
    assert_eq!(raw[1], 126);
    assert_eq!(u16::from_be_bytes([raw[2], raw[3]]), 126);
    let second = 4 + 126;
    assert_eq!(raw[second + 1], 127);
    assert_eq!(
        u64::from_be_bytes(raw[second + 2..second + 10].try_into().unwrap()),
        65_536
    );

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let rpc = RpcNode::new(Chain::new(hash_bytes(b"test", &[b"beacon"])));
    let server_thread = std::thread::spawn(move || {
        let (mut server, _) = listener.accept().unwrap();
        rpc.serve_explorer_websocket_once(&mut server).unwrap();
    });
    let mut client = TcpStream::connect(addr).unwrap();
    client.write_all(&[0x88, 0]).unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let mut close_response = Vec::new();
    client.read_to_end(&mut close_response).unwrap();
    server_thread.join().unwrap();
    assert_eq!(close_response, vec![0x88, 0]);

    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
}

#[test]
fn websocket_query_helper_decodes_auth_tokens() {
    let (path, token) = split_path_and_auth_token("/explorer/ws?x=1&token=a%20b+z%2f%ZZ");
    assert_eq!(path, "/explorer/ws");
    assert_eq!(token.as_deref(), Some("a b z/%ZZ"));
}

fn read_single_websocket_frame(frame: &[u8]) -> std::io::Result<Option<String>> {
    use std::io::Write;
    use std::net::{Shutdown, TcpListener, TcpStream};

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let mut client = TcpStream::connect(addr)?;
    let (mut server, _) = listener.accept()?;
    client.write_all(frame)?;
    client.shutdown(Shutdown::Write)?;
    read_websocket_text_frame(&mut server)
}
