use super::execute_test_cli_args;

#[test]
fn miner_start_requires_real_cuda_readiness_for_cuda_devices() {
    #[cfg(not(feature = "cuda-kernels"))]
    assert_eq!(
        execute_test_cli_args(&[
            "miner",
            "check",
            "--wallet",
            "miner.key",
            "--device",
            "cuda:0",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
        ])
        .unwrap_err()
        .to_string(),
        "cuda kernels not compiled"
    );

    #[cfg(feature = "cuda-kernels")]
    {
        let device_count = crate::runtime::cuda_device_count().unwrap_or(0);
        if device_count > 0 {
            let report = execute_test_cli_args(&[
                "miner",
                "check",
                "--wallet",
                "miner.key",
                "--device",
                "cuda:0",
                "--node",
                "/ip4/127.0.0.1/tcp/4001",
            ])
            .unwrap();
            let device_count_field = device_count.to_string();
            let suite_hash = crate::hash::hex(&crate::conformance::conformance_suite_hash());
            super::assert_report_fields(
                &report,
                &[
                    ("command", "miner_start"),
                    ("device", "cuda:0"),
                    ("device_backend", "cuda"),
                    ("gpu_backend_ready", "true"),
                    ("cuda_kernels_compiled", "true"),
                    ("cuda_device_index", "0"),
                    ("cuda_device_count", device_count_field.as_str()),
                    ("cuda_conformance_passed", "true"),
                    ("cuda_conformance_suite_hash", suite_hash.as_str()),
                ],
            );
        }
        let unavailable_device = format!("cuda:{device_count}");
        assert!(
            execute_test_cli_args(&[
                "miner",
                "check",
                "--wallet",
                "miner.key",
                "--device",
                &unavailable_device,
                "--node",
                "/ip4/127.0.0.1/tcp/4001",
            ])
            .is_err()
        );
    }
}
