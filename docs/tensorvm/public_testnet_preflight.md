# TensorVM Public Testnet Preflight

Status: local launch-readiness check only; this is not public-testnet evidence.

The preflight manifest records whether a proposed public TensorVM run has the local shape and deployment
plan needed before starting the external 7-day testnet. Passing preflight does not prove the run happened.
The post-run evidence bundle in [`public_testnet_evidence.md`](public_testnet_evidence.md) is still required
before the MVP can be called fully complete.

## Checked Gates

The local preflight report checks:

- at least 10 planned miners and 5 planned validators under the default public criteria
- positive miner and validator stakes
- funded faucet configuration
- available CUDA kernels for the claimed GPU mining path and one CUDA-ready miner-start check per
  planned miner
- production libp2p runtime plan with one libp2p-ready node per planned miner and validator, plus
  discovery, gossip, request/response, and DoS controls
- exactly one public HTTPS RPC, explorer, faucet, and telemetry service plan
- distinct service endpoint identifiers, health URLs, content URLs, auth, and rate limiting

Public HTTPS service hosts must be externally reachable names or addresses with well-formed authorities,
and each service's health and content URLs must use the same HTTPS authority. The local checker rejects
userinfo, whitespace, invalid DNS host labels, single-label DNS hosts, invalid ports, malformed bracketed
IPv6 authorities, localhost, `.local`, `.localhost`, `.test`, `.example`, `.invalid`, RFC example
domains, loopback, unspecified, private, and link-local IP addresses, including bracketed IPv6 loopback
literals. Direct IP literals from documentation, shared-address, benchmarking, multicast, or reserved
ranges are also rejected. RPC, explorer, faucet, and telemetry plans must use distinct endpoint IDs,
distinct health URLs, and distinct content URLs; launch readiness requires exactly one ready service plan
for each of those four surfaces. Missing, duplicate, reused-URL, or extra service plans do not satisfy
launch readiness.

## Manifest Format

The parser is `parse_public_testnet_preflight_manifest`. Blank lines and `#` comments are ignored. Hash
values are 64-character hex strings with an optional `0x` prefix. Boolean values are `true` or `false`.
Field names must be exact with no leading or trailing whitespace around the key before `=`. Scalar
manifest fields must appear exactly once, and scalar values are parsed exactly with no leading or trailing
whitespace; `service=...` is the only repeated field. Each `service=...` record must contain exactly eight
comma-separated, nonempty fields; preflight service values are not trimmed, so leading or trailing
whitespace in any service value is invalid. Deployment readiness requires exactly one ready `service=...`
record for each RPC, explorer, faucet, and telemetry surface.

```text
version=tensor-vm-public-testnet-preflight-v1
miner_count=10
validator_count=5
miner_stake=100
validator_stake=10000
faucet_balance=1000000
faucet_drip=100
cuda_kernels_available=true
cuda_ready_miner_count=10
libp2p_ready_node_count=15
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
service=rpc,<endpoint-id-hex>,https://rpc.tensorvm.net/health,/health,https://rpc.tensorvm.net/chain/head,/chain/head,true,true
service=explorer,<endpoint-id-hex>,https://explorer.tensorvm.net/health,/health,https://explorer.tensorvm.net/explorer,/explorer,true,true
service=faucet,<endpoint-id-hex>,https://faucet.tensorvm.net/health,/health,https://faucet.tensorvm.net/faucet/page,/faucet/page,true,true
service=telemetry,<endpoint-id-hex>,https://telemetry.tensorvm.net/health,/health,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,true,true
```

`cuda_ready_miner_count` must match `miner_count` and should be derived from successful
`tvmd miner check --device cuda:N` readiness checks on each planned public miner host, not from a copied
boolean.
`libp2p_ready_node_count` must match `miner_count + validator_count` and should be derived from successful
`tvmd node check --p2p-listen <multiaddr> --data-dir <path>` checks on every planned public miner
and validator. The readiness command loads the initialized node store and durable peer book, starts the
same mandatory rust-libp2p control plane used by `tvmd node serve`, reports `libp2p_ready=true`, and
exits.

Each `service=...` line records the service kind, endpoint ID, public health URL, health path, public
content URL, required content path, auth flag, and rate-limit flag. The health URL path must match the
health path exactly, must be a concrete non-root path, and must not include a query string or fragment.
The content URL path must match the required public surface for that service exactly, also as a concrete
non-root path without a query string or fragment:
`/chain/head`, `/explorer`, `/faucet/page`, or `/telemetry/dashboard`. The content URL authority must
match the health URL authority so a preflight manifest cannot combine health checks from one deployed
service with content evidence from another host. Endpoint IDs, health URLs, and content URLs must be
distinct across the RPC, explorer, faucet, and telemetry service plans, and extra duplicate service plans
are rejected by the public service plan gate.

The CLI reads a manifest file and reports launch readiness:

```bash
tvmd public preflight docs/tensorvm/public-testnet.preflight
```

[`public-testnet.preflight`](public-testnet.preflight) is checked into docs at the spec-referenced
path. It intentionally uses reserved placeholder hosts and must report `deployment_plan_ready=false` until
owned public HTTPS hosts and endpoint IDs replace it.

A checked deployment scaffold is available under [`../../deploy/tensorvm`](../../deploy/tensorvm):

- `systemd/tensorvm.service` runs the explicit `tvmd` binary target with required `--p2p-listen`
- `env/public-testnet.env.example` records the service listen address, libp2p multiaddr, data directory,
  auth token, and request limit
- `nginx/tensorvm.conf` publishes RPC, explorer, faucet, and telemetry hostnames over external HTTPS
- `manifests/public-testnet.preflight.example` is a concrete preflight manifest shape for replacement by
  real public endpoint IDs and hostnames

The example manifest can be checked from the repository root with:

```bash
cargo run -p tensor_vm --bin tvmd -- public preflight deploy/tensorvm/manifests/public-testnet.preflight.example
```

The checked deployment example intentionally uses reserved placeholder hostnames, so it parses but reports
`deployment_plan_ready=false` until those hosts and endpoint IDs are replaced with owned public HTTPS
authorities.

The reference service process can be prepared and launched with:

```bash
tvmd node init --data-dir /var/lib/tensorvm
tvmd node peer add --data-dir /var/lib/tensorvm --peer-id "$BOOTSTRAP_PEER_ID" --address /dns/bootstrap.tensorvm.net/tcp/4001
tvmd node check --p2p-listen /ip4/0.0.0.0/tcp/4001 --data-dir /var/lib/tensorvm
tvmd node serve --listen 0.0.0.0:8545 --p2p-listen /ip4/0.0.0.0/tcp/4001 --data-dir /var/lib/tensorvm --auth-token service-token --max-requests 0
```

The service exposes `GET /health` plus scoped `GET /rpc/health`, `GET /explorer/health`,
`GET /faucet/health`, and `GET /telemetry/health` endpoints for external monitors. The generic `/health`
path is suitable when each public service hostname routes to the same TensorVM service process. It also
exposes the content surfaces later required by public evidence: `GET /chain/head`, `GET /explorer`,
`GET /faucet/page`, and `GET /telemetry/dashboard`.

The output is a line-oriented readiness report. `public_testnet_preflight_ready=true` only means the
planned run has the required local shape and deployment plan; it still does not prove an external run
has happened. Failed launches can be diagnosed from the individual gate fields:

```text
public_testnet_preflight_ready=true
local_shape_ready=true
deployment_plan_ready=true
miners=10
validators=5
required_blocks=100800
required_miners=true
required_validators=true
positive_stakes=true
funded_faucet=true
cuda_kernels_available=true
cuda_ready_miner_count=10
cuda_ready_miners=true
libp2p_ready_node_count=15
libp2p_ready_nodes=true
production_libp2p_runtime=true
rpc_service_plan=true
explorer_service_plan=true
faucet_service_plan=true
telemetry_service_plan=true
public_service_content_planned=true
public_services_planned=true
```
