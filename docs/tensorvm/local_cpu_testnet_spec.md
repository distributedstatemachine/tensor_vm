# TensorVM Local CPU Testnet Spec

This is the first focused implementation target for TensorVM: a full local testnet that runs on CPU and
can be brought up entirely with Docker Compose. It is intentionally narrower than the public-testnet MVP
scope in [`mvp_spec.md`](mvp_spec.md): no CUDA, no public endpoints, no 7-day external operator evidence,
and no systemd/nginx production deployment.

The local CPU testnet is still a real protocol deployment. It must use the canonical CPU backend, durable
per-operator state, and the mandatory rust-libp2p runtime. It must not use simulations, direct in-memory
propagation, local-only networking shims, or single-participant shortcuts.

## Goal

From a clean checkout, one Docker Compose command should start a complete local TensorVM testnet with:

```text
10 miner operators
5 validator operators
deterministic CPU tensor execution for every miner
one libp2p node identity and data directory per operator
real libp2p discovery, gossip, and request/response paths between containers
block production, receipt submission, validation, attestation, settlement, rewards, and telemetry
live synthetic CPU jobs that continue advancing blocks after the seeded chain is available
local RPC, TensorVM explorer WebSocket data, faucet, and telemetry surfaces reachable from the host
standalone browser explorer service reachable from the host
```

The default operator shape is 10 miners and 5 validators because it matches the public-testnet minimum
shape without requiring public infrastructure. Smaller smoke-test profiles may exist, but they do not
satisfy this spec.

## Non-Goals

This spec does not require:

```text
CUDA kernels or GPU devices
public DNS, public TLS, or nginx
systemd deployment
independent external operators
7-day public-run evidence
public-testnet reward claims
mainnet security claims
```

The local run may produce useful engineering evidence, but it is not public-testnet evidence.

## Required Artifacts

The implementation must add a checked local deployment bundle under:

```text
deploy/tensorvm/local-cpu/
```

The bundle must include:

```text
docker-compose.yml
Dockerfile
README.md
env/local-cpu.env.example
scripts/entrypoint.sh
scripts/check-local-testnet.sh
scripts/check-restart-continuity.sh
scripts/check-rolling-restart-continuity.sh
```

`docker-compose.yml` may be generated from a template, but the rendered file that users run must be
checked in and reviewable. `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
must pass from the repository root.

## Docker Image

The local CPU image must build the TensorVM daemon with default features only:

```bash
cargo build -p tensor_vm --release
```

The image must not build with `--features cuda-kernels`, require NVIDIA container runtime support, mount
GPU devices, or set CUDA-specific environment variables. Any miner container that reports readiness under
this spec must report the CPU backend, not a CUDA backend.

The image must contain the `tvmd` and `tensorvm-explorer` binaries and any local entrypoint scripts
required to initialize state, seed bootstrap peers, start libp2p, run the assigned miner or validator
role, and serve the browser explorer.

## Operator Topology

The default Compose deployment must define all 15 counted operators explicitly or through a checked,
deterministic rendered configuration:

```text
miner-00
miner-01
miner-02
miner-03
miner-04
miner-05
miner-06
miner-07
miner-08
miner-09
validator-00
validator-01
validator-02
validator-03
validator-04
```

Each operator service must have:

```text
a stable operator ID
a stable local wallet/key identity
a stable libp2p peer identity
a unique data volume
a unique internal RPC listen address or port
a unique internal libp2p listen multiaddr
a role of exactly miner or validator
```

`miner-00` may also act as the local bootstrap and host-facing gateway, but it still counts as a miner
operator and must run the same miner role as the other miner operators. The other 14 operators must seed
their peer books from `miner-00` by using its libp2p peer ID and Docker DNS address.

Checked deterministic development keys are acceptable for this local-only bundle if they are clearly
marked as non-secret and unusable for public deployments.

## Networking

The Compose file must create one private bridge network, for example:

```text
tensorvm-local
```

All operator-to-operator protocol traffic must use Docker-network libp2p multiaddrs. Host networking is
not required. The default host-published ports should be limited to the gateway surfaces exposed by
`miner-00` plus the standalone explorer:

```text
8545/tcp  local HTTP RPC, TensorVM explorer WebSocket data, faucet, and telemetry
4001/tcp  optional host-visible libp2p bootstrap port
8080/tcp  standalone browser explorer
```

The default standalone explorer host port is `8080`, but the Compose bundle may support a local override
when that host port is already in use.

Every counted operator must run the mandatory libp2p control plane with TensorVM's configured gossip
topics and request/response protocols. A container is not ready unless its readiness output includes:

```text
p2p_runtime=libp2p
node_store_ready=true
libp2p_ready=true
```

## Runtime Flow

Each operator container must perform the same high-level startup sequence:

```text
initialize or load its durable node store
load its stable wallet and libp2p identity
seed the bootstrap peer book, except for miner-00
start the mandatory libp2p control plane
start its role command as `tvmd miner run` or `tvmd validator run`; `validator-00` is the only service
with the local timed synthetic job producer flag, and `validator-00` plus delayed `validator-01` are
validator block proposers
report readiness only after the role process and libp2p runtime are live
```

Miner containers must start with:

```text
device=cpu
```

Validator containers must connect through libp2p and validate receipts using the canonical verifier
paths. Role loops may be supervised by an entrypoint script, but the role loops must use real `tvmd`
protocol paths and persisted state. The readiness file and `tvmd node status` output must expose the
actual long-running role command so the checker can reject operators that fall back to a generic service
entrypoint. The status output must also expose role-loop counters, local-producer mode, network-applied
block counters for non-producers, the runtime-observed libp2p connected peer count, job, receipt,
attestation, block, and block-payload gossip counters, decoded network-event ingestion counters, decoded
block, job, receipt, and attestation payload application counters, the latest observed block and block-payload
height/hash, and the bounded observed block-hash set so the first gate proves every operator is connected and receiving live consensus
announcements for the convergence target, not merely configured.

After bootstrap, the host-facing gateway node must keep generating deterministic synthetic CPU work, and
non-producer operators must only apply live blocks after decoding a p2p block payload and verifying it
against the shared chain path. Each live block must come from the normal protocol path: a
generated TensorWork job, miner receipts, validator attestations, epoch settlement, proposer selection, and
block-finality votes. A local run that only serves the seeded two-block snapshot does not satisfy this
spec.

## Local Services

The local gateway exposed by `miner-00` must serve these routes from the host:

```text
GET /health
GET /rpc/health
GET /chain/head
GET /jobs/current
GET /explorer/health
GET /explorer
GET /explorer/overview
GET /explorer/miners
GET /explorer/validators
GET /explorer/jobs
GET /explorer/receipts/latest/:limit
WS  /explorer/ws
GET /faucet/health
GET /faucet/page
GET /telemetry/health
GET /telemetry/dashboard
```

The default explorer UI must be a terminal-style browser surface backed by the `tensor_vm_explorer`
`ui` feature and its Ratzilla/Ratatui WASM path. It must not depend on a checked static chain dump.
The TensorVM node must expose `/explorer/ws`, and the standalone explorer must poll that WebSocket for
the data it renders. The gateway does not need public TLS for this spec. Local HTTP/WebSocket is enough.

## Acceptance Gates

Gate L0 is still the first executable gate for implementation work:

```bash
cargo test -p tensor_vm local_testnet --release
```

After Gate L0 passes, the Docker Compose gate for this spec is:

```bash
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml down -v
```

`check-local-testnet.sh` must fail unless it observes all of the following from the running Compose
deployment:

```text
10 ready miner operators
5 ready validator operators
15 distinct operator IDs
15 distinct libp2p peer IDs derived from stable operator identities
15 distinct durable data directories or volumes
15 libp2p-ready nodes
10 CPU-ready miners
0 CUDA-required miners
all miner operators report `runtime_command=miner_run`
all validator operators report `runtime_command=validator_run`
`validator-00` reports local timed synthetic job production, `validator-00` and delayed `validator-01`
report local validator block-proposer mode, and every miner reports no block-production capability
all operators report `chain_profile=local_cpu` / `role_chain_profile=local_cpu`
all operators report live role-loop readiness, role command, produced-block counters, and latest role height
at least one finalized block after startup
chain height and block count advance past the seeded two-block baseline
synthetic post-startup jobs, receipts, and settled receipts are visible through explorer data
at least one settled matmul TensorWork receipt
at least one settled LinearTrainingStep receipt
post-seed receipt details name TensorOp and LinearTrainingStep primitive types
`tvmd node block` exposes finalized live block-height receipt IDs and primitive counts for both TensorOp
and LinearTrainingStep work
validator attestations for settled receipts
miner and validator rewards credited from settled TensorWork
tensor data available through the local tensor-server path
gateway health, chain head, explorer, faucet, and telemetry routes reachable from the host
standalone explorer route reachable from the host
standalone explorer page configured to poll the TensorVM `/explorer/ws` endpoint
all 15 operator node stores advanced past the seed, reported role status and live chain counters, and
reported the same first live finalized block hash plus the same finalized common-head block hash at the
bounded convergence height
only validator-00 reported local timed synthetic job production, only validator-00 and delayed validator-01
reported local validator block-proposer mode, miners reported no local block-production capability, and
every other counted operator reported network-applied block progress from decoded p2p block payloads
every non-producer reported decoded network-event ingestion for block headers, jobs, receipts, and
attestations, with zero invalid network events
every non-producer reported decoded block, job, receipt, and attestation payload application through the chain engine
at least one role reported decoded external randomness beacon ingestion and application through the same
network event path, and the aggregate checker output reported
`live_role_network_external_randomness_beacons_applied` greater than zero
all 15 operator node stores returned the finalized local-head checkpoint hash and state root after that
checkpoint was observed through p2p block gossip
all 15 operators observed that checkpoint block hash through libp2p block gossip
all 15 operator node stores reported nonempty block-log roots
restarted operators repair torn snapshot/block-log state from persisted chain state before readiness
```

The check must also verify that the run reports itself as local-only:

```text
public_evidence_full_spec=false
independently_checkable=false
```

## Restart Gate

The Compose deployment must survive a local restart test before this spec is complete:

```bash
deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh
```

The rolling continuity script must invoke the single-service continuity gate for every counted operator by
default. Each pass must capture pre-restart and post-restart peer IDs, heights, block counts, state roots,
block-log roots, and a finalized common-head block. It must prove that the restarted operator reused its
original durable state and libp2p identity, rejoined the local network, advanced height, block count, state
root, and block-log root, preserved the pre-restart finalized common head and state root on every operator,
and continued producing finalized blocks. A smaller explicit service list may be used for smoke checks, but
it does not satisfy this spec.

The restart path must also tolerate an interrupted local write where the snapshot and block log disagree.
`tvmd node init` must validate the complete node store, recover snapshot and block-log files from the
persisted `chain.state` file when that state is valid, and fail readiness if neither the complete store nor
chain-state recovery is valid.

## Completion Criteria

This spec is complete only when:

```text
Gate L0 passes
the checked Compose config is valid
the default Compose deployment starts all 15 operators
the default Compose deployment passes the functional check script
the rolling all-operator restart gate passes
the local run uses CPU only
every counted operator uses mandatory libp2p
the run produces block, receipt, attestation, settlement, reward, data-availability, and telemetry evidence
the standalone explorer is started by Docker Compose and polls live TensorVM node data over `/explorer/ws`
docs/tensorvm/implementation_status.md records the successful commands and observed counts
docs/tensorvm/coverage_matrix.md maps the local CPU Compose gates to tests or check scripts
```

Until all criteria above pass, the local CPU testnet is incomplete. Even after completion, it remains a
local engineering milestone and does not satisfy the public 7-day testnet gate.
