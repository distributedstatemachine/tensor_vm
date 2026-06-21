# TensorVM Local CPU Compose Testnet

This bundle is the Docker Compose deployment target for
[`docs/tensorvm/local_cpu_testnet_spec.md`](../../../docs/tensorvm/local_cpu_testnet_spec.md). It is local
only, CPU only, and uses one container per counted operator.

## Services

The default topology defines 10 miners, 5 validators, and a standalone explorer UI:

```text
miner-00 ... miner-09
validator-00 ... validator-04
explorer
```

`miner-00` is also the local bootstrap and host-facing gateway. It publishes:

```text
127.0.0.1:8545 -> local RPC, explorer, faucet, and telemetry HTTP routes
127.0.0.1:4001 -> optional host-visible libp2p bootstrap port
127.0.0.1:8080 -> standalone TensorVM explorer
```

Every operator container initializes a durable node store, starts with a stable local operator ID, uses a
distinct data volume, derives a stable libp2p identity seed from that operator ID, runs the mandatory
libp2p readiness path, and then execs its role command: all miners run `tvmd miner run`, all validators
run `tvmd validator run`, `validator-00` carries the single local timed synthetic job producer flag, and
`validator-00` plus delayed `validator-01` carry validator block-proposer flags.
Miner containers also run the CPU miner readiness command with `--device cpu`; validators run the validator
readiness command. Every operator seeds the same deterministic local CPU chain and keeps producing live
synthetic CPU jobs from that shared base, while `miner-00` exposes the host-facing gateway routes.

The standalone explorer is served by `tensorvm-explorer serve`. It polls `miner-00` through
`ws://127.0.0.1:8545/explorer/ws?token=local-cpu-testnet-token` for live chain data.
If `8080` is already in use, set `TENSORVM_LOCAL_CPU_EXPLORER_PORT` before running Compose and the check
script.

The check script waits for `/chain/head` and `/explorer/overview` to move past the seeded two-block
snapshot, including new jobs, receipts, settled receipts, model-count advancement, validator-attestation
growth, per-receipt validator-attestation details, named post-seed TensorOp and LinearTrainingStep
receipts, live tensor descriptor/row/chunk/opening fetches, and reward growth from live synthetic work. It
also runs `tvmd node status` in every operator container and
fails unless all 15 node stores advance past the seed, report role-specific status and the expected
role-runtime command, expose live role-loop counters, the single local synthetic job producer flag,
delayed multi-validator proposer counters, non-producer
network-applied block counters, decoded network-event ingestion counters, decoded job, receipt, and
attestation payload application through the chain engine, positive network-applied external randomness
beacon counters, real libp2p connected-peer counts, observed job,
receipt, attestation, and block gossip counters, latest observed block heights and hashes, the bounded
observed block-hash set, and chain counters, report the same first live finalized block hash, and return
the same finalized common-head block hash through `tvmd node block`.
It also selects miner-00's latest finalized p2p-observed head from the block-gossip set, then fails unless
every operator can return that finalized block hash and state root, has observed that network head through
libp2p block gossip, and reports a nonempty block-log root.

## Commands

From the repository root:

```bash
cargo test -p tensor_vm local_testnet --release
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml down -v
```

The Docker image builds `tensor_vm` with default features only. It does not enable `cuda-kernels`, mount
GPU devices, or require the NVIDIA container runtime.

## Evidence Boundary

The local Compose run is engineering evidence for the CPU local-testnet milestone. It is not
independently checkable public-testnet evidence and must continue to report:

```text
live_role_network_external_randomness_beacons_applied>0
live_external_randomness_beacon_evidence=true
public_evidence_full_spec=false
independently_checkable=false
```

The rolling restart script invokes the restart-continuity gate once per counted operator by default. Each
pass captures all operator peer IDs, heights, block counts, state roots, block-log roots, and a finalized
common head before restarting one requested service. After the restart and local-testnet check, it fails
unless the restarted service keeps its libp2p peer ID, height, block count, state root, and block-log root
advance, the pre-restart finalized common head and state root are still present on every operator, and new
finalized blocks are observed. Pass explicit service names to run a smaller smoke subset.

On restart, `tvmd node init` validates the complete node store. If a previous write left the snapshot and
block log out of sync, the service rewrites them from the persisted `chain.state` file before reporting
local readiness.
