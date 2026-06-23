# TensorVM Public Testnet Evidence

Status: no complete external public-testnet evidence bundle is available yet.

This document is the publication target for the independently checkable evidence bundle required before
the TensorVM MVP can be called fully complete. A complete bundle must be produced from an external public
run, not from the local harness.

For pre-run launch readiness, use [`public_testnet_preflight.md`](public_testnet_preflight.md). A passing
preflight report is not a substitute for this post-run evidence bundle.

A deployment runbook for collecting, validating, and publishing the external-run evidence lives at
[`../../deploy/tensorvm/RUNBOOK.md`](../../deploy/tensorvm/RUNBOOK.md).

[`public-testnet.evidence`](public-testnet.evidence) is checked in at the spec-referenced validation path
as a pending manifest. It parses and reports fields for a placeholder short run, but it must remain
`public_evidence_full_spec=false` until every ID, URI, root, count, and signature is replaced with records
from a real external 7-day public run.

A checked example manifest lives at
[`../../deploy/tensorvm/manifests/public-testnet.evidence.example`](../../deploy/tensorvm/manifests/public-testnet.evidence.example).
It is useful for validating the post-run manifest shape, signature domains, and reporting fields, but it is
deliberately uses special-use placeholder hosts and only a 60-second, 10-block, 2-miner, 1-validator
sample, so it is not independently checkable or full-spec public-testnet evidence.

## Required Bundle

A complete evidence bundle must include:

- a public `https://`, `ipfs://`, or `ar://` location for the evidence manifest
- one manifest signature record for the current manifest format
- signed independent auditor records observed at or after the signed run end
- signed wall-clock run window covering the full 7-day run
- signed miner and validator heartbeat history for the full run
- independent operator identity or attestation records
- signed block-history summary root for the full 7-day run
- signed finality-history summary root for the full 7-day run
- signed production libp2p network-observation records for every counted public operator and their
  aggregate summary root
- signed data-availability measurement summary root for checked tensor receipts
- signed invalid-work submission and rejection evidence
- signed reward-settlement records for verified TensorWork
- signed deployed detection-measurement records
- signed validator VRF lifecycle records proving checked validator rewards reached reveal-complete
  lifecycle state before claimability
- exactly one signed external artifact locator for the raw supporting records behind each
  block/finality/libp2p/randomness/data-availability/invalid-work/reward-settlement/detection-measurement/validator-vrf-lifecycle summary root,
  with distinct artifact URIs across those record kinds
- proof that production libp2p was used for peer discovery, gossip, and request/response propagation,
  with one signed observation record per counted public miner or validator operator
- external HTTPS URLs, health paths, reachability records, content paths, and signed content-root
  observations for deployed RPC, explorer, faucet, and telemetry services
- distinct deployed endpoint IDs and distinct signed service-content roots for the RPC, explorer, faucet,
  and telemetry service surfaces

A public `https://` evidence URI must use a well-formed external host authority. The local validator
rejects userinfo, whitespace, invalid DNS host labels, single-label DNS hosts, invalid ports, malformed
bracketed IPv6 authorities, localhost, `.local`, `.localhost`, `.test`, `.example`, `.invalid`, RFC
example domains, loopback, unspecified, private, link-local, documentation, shared-address, benchmarking,
multicast, reserved IP addresses, missing or root-only HTTPS paths, HTTPS query strings, and HTTPS fragments.
`ipfs://` and `ar://` publication URIs must start with a well-formed content identifier segment using
only ASCII alphanumerics, `-`, or `_`. Optional path segments after the identifier may use ASCII
alphanumerics, `.`, `-`, and `_`, but empty segments, `.`/`..`, query strings, fragments, backslashes, raw
whitespace, and control characters are rejected.

## Current Repository Evidence

The local reference crate exposes typed validation for this future bundle through
`PublicTestnetEvidenceBundle`. The validator intentionally separates:

- `PublicTestnetRunEvidence`, which checks run-level protocol evidence
- `PublicTestnetEvidenceBundle`, which additionally checks publication, signatures, auditors, and
  independently checkable supporting records

The current local reference implementation and docs do not satisfy this bundle requirement. The manifest
validator requires signed node-heartbeat summaries, signed operator identity attestations, signed
service-health summaries, signed public service-content summaries, special-use DNS placeholder rejection,
signed independent auditor records
whose auditor IDs differ from the manifest signer and whose observation timestamps are at or after the
signed run end, and an external publication URI. It verifies
a manifest publication signature over the bundle ID, public URI, exact manifest signature count, and
independent auditor count. It also verifies signed auditor records over the bundle ID, public URI, external
audit URI,
auditor ID, and observation time, plus a signed run-window record over the manifest bundle ID, start time,
end time, and observed block count. It verifies signed supporting-record roots for block history, finality
history, production libp2p observations, data-availability measurements, invalid-work rejections, reward
settlements, deployed detection measurements, and validator VRF lifecycle records. It also requires signed external artifact locators for the raw supporting records behind
each summary root with distinct artifact URIs across those record kinds, and it derives `external_operator_evidence` from signed operator identity attestation
records that match the signed node-heartbeat records. The operator identity attestation count must exactly
match the repeated `operator=...` line count and the valid signed records; missing, invalid, duplicate, or
extra operator records do not count. These local checks are still only evidence-format validation until an
external run publishes real records. Run-level finality and data-availability counters must also be
internally consistent: finalized blocks cannot exceed observed blocks, and available receipts cannot exceed
checked receipts. The run-derived supporting-record counts must be exact, not padded: block-history and
finality-history record counts must match observed blocks, use distinct nonzero block roots, agree on the
block root for each block number, and contain a finalized status count equal to `finalized_blocks`,
data-availability measurement count must match
checked receipts and use distinct nonzero receipt roots, invalid-work rejection record count must match
invalid receipts submitted and use distinct nonzero receipt roots,
detection-measurement record count must be positive for full-spec evidence, validator-VRF-lifecycle
record count must match checked receipts, reward-settlement raw records must use distinct nonzero receipt
roots and nonzero miner/validator IDs, and the
production-libp2p network-runtime observation count must match the counted independent miner and validator
operator total exactly and use distinct public peer IDs and listen multiaddrs. Raw randomness-beacon
records must be accepted public `drand-v1` or `validator-vrf-v1` records covering each observed block
exactly once with distinct source/round pairs.

## Manifest Format

External evidence can be represented as a line-oriented manifest parsed by
`parse_public_testnet_evidence_manifest`. Blank lines and `#` comments are ignored. Hash values are
64-character hex strings with an optional `0x` prefix. Boolean values are `true` or `false`. Field names
must be exact with no leading or trailing whitespace around the key before `=`. Scalar manifest fields must
appear exactly once, and scalar values are parsed exactly with no leading or trailing whitespace; repeated
record fields are allowed only for `auditor`, `record_artifact`, `operator`, `network_runtime_observation`,
`block_history_record`, `finality_history_record`, `randomness_beacon_record`, `data_availability_measurement`, `invalid_work_rejection`,
`reward_settlement`, `detection_measurement`, `validator_vrf_lifecycle`, `node`, `service`, and `service_content`. The comma-separated values inside those
repeated public-evidence records must also be exact, nonempty, and free of leading or trailing whitespace. For `record_artifact`, the full
independently checkable gate requires exactly one valid line for each required supporting-record kind,
distinct artifact URIs across those required kinds, and rejects extra artifact locators. The manifest signature covers the bundle ID, public URI, manifest
signature count, and independent auditor count. The current manifest format carries exactly one
`manifest_signature` field, so `manifest_signature_count` must be `1`; claimed extra manifest signatures
cannot count until multiple signature records are modeled.
Auditor signatures cover the bundle ID, public URI, auditor ID, external audit URI, and observation time.
Counted auditor records must be observed at or after the signed run-window end so an audit cannot count
before the public run has completed. The independently checkable evidence gate requires exactly
`independent_auditor_count` valid signed `auditor=` records; missing, invalid, or extra auditor records
do not satisfy the gate.
Block, finality, network-runtime, data-availability, invalid-work, reward-settlement, and
detection-measurement signatures cover the bundle ID, record-set kind, record-set root, and record count. The run-window signature covers the
bundle ID, Unix start time, Unix end time, and observed block count. Heartbeat signatures cover the node
role, address, operator ID, first/last observed block, and heartbeat count. Operator identity signatures
cover the node role, node address, operator ID, external identity URI, and observation time.
Service-health signatures cover the service kind, endpoint ID, public URL, health path, first/last observed
block, reachable observation count, and signed health-check count. Supporting-artifact signatures cover the
bundle ID, record-set kind, external artifact URI, record root, and record count. Service-content
signatures cover the service kind, endpoint ID, public URL, content path, content root, observation time,
and minimum observed content bytes; counted public service content must prove at least 64 observed bytes.
Network-runtime observation signatures cover the operator ID and raw observation root derived from the
public peer ID, public libp2p listen address, observation time, discovery/bootstrap counts,
Gossipsub/request-response protocol counts, and DoS-control limits.
The RPC, explorer, faucet, and telemetry services must have distinct endpoint IDs and distinct deployed
content roots. Reusing a health endpoint ID or content root across service kinds does not satisfy the
deployed public-service gate.
Service URLs, service-content URLs, supporting artifact HTTPS URIs,
auditor HTTPS URIs, and operator identity HTTPS URIs must use well-formed external host authorities;
userinfo, whitespace, invalid DNS host labels, single-label DNS hosts, invalid ports, malformed bracketed
IPv6 authorities, localhost, `.local`, `.localhost`, `.test`, `.example`, `.invalid`, RFC example
domains, loopback, private, link-local, unspecified, documentation, shared-address, benchmarking,
multicast, reserved IP hosts, missing or root-only HTTPS paths, HTTPS query strings, and HTTPS fragments are rejected.
Supporting artifact, auditor, and operator identity URIs may also use `ipfs://` or `ar://` identifiers
with the same well-formed first-segment and optional path-segment rule and no query strings, fragments,
backslashes, raw whitespace, or control characters.
The service-health URL path must be concrete and non-root, must match the signed health path exactly, and
must not include a query string or fragment. Public service-content URLs use the same concrete non-root
exact-path rule for their required content path.
Counted miner and validator operator and node-address sets must be disjoint; the same operator ID or node
address cannot satisfy both role minima in a public-run bundle, and repeated node addresses cannot inflate a
single role's independent participant count. Counted participants must admit a one-to-one matching between
live operator IDs and live node addresses; separate unique operator and address totals are not sufficient
without that matching. The matching is criteria-aware: if greedy role ordering or address choice would miss
a satisfiable set, the validator must use an independent operator/address set that satisfies the requested
miner and validator minima when one exists.
For a run to satisfy the public gate, every counted miner/validator heartbeat summary must span the full
observed block range and carry at least one signed heartbeat per observed block. Counted operator identity
attestations must have observation timestamps inside the signed run window, match the selected counted
operator/address pairs from live node-heartbeat records, and exactly match the
`operator_identity_attestation_records` manifest count and the repeated `operator=...` line count.
The public service gate counts exactly one service-health record and exactly one service-content record
for each RPC, explorer, faucet, and telemetry surface; extra service-health or service-content records do
not satisfy the deployed public-service gate. Counted service-health URLs and service-content URLs must be
distinct across service kinds. Counted service-content records must have observation timestamps inside the
signed run window. Every counted service health summary must likewise span the full observed block range
and carry at least one reachable observation and one signed health check per observed block. The reachable
observation count must not exceed the signed health-check count.
Finalized-block and available-receipt totals must not exceed their corresponding observed-block and
checked-receipt denominators; capped percentage output does not make impossible counter sets satisfy the
public gate. Signed overcounts or undercounts for block-history, finality-history, data-availability,
invalid-work, or network-runtime summary records do not satisfy the independently checkable gate. The
network-runtime summary must also be backed by exactly one valid signed `network_runtime_observation`
record for every counted public miner and validator operator; each observation must name that operator,
use a valid libp2p peer ID, use a public nonzero-TCP listen multiaddr, have an observation timestamp inside
the signed run window, use a distinct peer ID and listen multiaddr, and aggregate to the signed
network-runtime root.
The reference service process serves `GET /health` for shared-host deployments and scoped
`GET /rpc/health`, `GET /explorer/health`, `GET /faucet/health`, and `GET /telemetry/health` endpoints
when operators publish distinct public service hostnames or paths. Public service-content observations
must bind the same endpoint IDs and HTTPS authorities as the corresponding health records to deployed
content roots for `GET /chain/head`, `GET /explorer`, `GET /faucet/page`, and
`GET /telemetry/dashboard`; other content paths or cross-authority content URLs do not satisfy the
deployed-service gate. Endpoint IDs and content roots must also be unique across the four service kinds.

```text
version=tensor-vm-public-testnet-evidence-v1
bundle_id=0x<64-hex>
public_uri=https://tensorvm.net/tensorvm/public-evidence.json
manifest_signer=<address-hex>
manifest_signature=<signature-hex>
manifest_signature_count=1
independent_auditor_count=1
auditor=<auditor-address-hex>,https://auditor.tensorvm.net/tensorvm/audit.json,<unix-seconds-at-or-after-run-end>,<auditor-signature-hex>
record_artifact=block-history,https://evidence.tensorvm.net/tensorvm/block-history.json,<history-root-hex>,100800,<artifact-signature-hex>
record_artifact=finality-history,https://evidence.tensorvm.net/tensorvm/finality-history.json,<finality-root-hex>,100800,<artifact-signature-hex>
record_artifact=network-runtime,https://evidence.tensorvm.net/tensorvm/network-runtime.json,<network-runtime-root-hex>,<operator-count>,<artifact-signature-hex>
record_artifact=randomness-beacon,https://evidence.tensorvm.net/tensorvm/randomness-beacon.json,<randomness-root-hex>,100800,<artifact-signature-hex>
record_artifact=data-availability,https://evidence.tensorvm.net/tensorvm/data-availability.json,<da-root-hex>,1000,<artifact-signature-hex>
record_artifact=invalid-work,https://evidence.tensorvm.net/tensorvm/invalid-work.json,<invalid-work-root-hex>,1,<artifact-signature-hex>
record_artifact=reward-settlement,https://evidence.tensorvm.net/tensorvm/reward-settlement.json,<reward-settlement-root-hex>,1,<artifact-signature-hex>
record_artifact=detection-measurement,https://evidence.tensorvm.net/tensorvm/detection-measurement.json,<detection-root-hex>,1,<artifact-signature-hex>
record_artifact=validator-vrf-lifecycle,https://evidence.tensorvm.net/tensorvm/validator-vrf-lifecycle.json,<validator-vrf-lifecycle-root-hex>,1000,<artifact-signature-hex>
block_history_records=100800
block_history_root=<history-root-hex>
block_history_signature=<history-signature-hex>
finality_history_records=100800
finality_history_root=<finality-root-hex>
finality_history_signature=<finality-signature-hex>
operator_identity_attestation_records=15
operator=miner,<address-hex>,<operator-id-hex>,https://operator-a.tensorvm.net/tensorvm.json,<unix-seconds>,<operator-signature-hex>
operator=validator,<address-hex>,<operator-id-hex>,https://operator-b.tensorvm.net/tensorvm.json,<unix-seconds>,<operator-signature-hex>
network_runtime_observation=<operator-id-hex>,<libp2p-peer-id>,/dns/node-a.tensorvm.net/tcp/4001,<unix-seconds>,5,4,2,1048576,10,128,60,<observation-root-hex>,<observation-signature-hex>
network_runtime_observation_records=<operator-count>
network_runtime_observation_root=<network-runtime-root-hex>
network_runtime_observation_signature=<network-runtime-signature-hex>
randomness_beacon_records=100800
randomness_beacon_root=<randomness-root-hex>
randomness_beacon_signature=<randomness-signature-hex>
block_history_record=<block>,<block-root-hex>
finality_history_record=<block>,<block-root-hex>,finalized|unfinalized
randomness_beacon_record=<source-id-hex>,1,<randomness-root-hex>,<proof-root-hex>,drand-v1,<observed-block>,accepted
data_availability_measurement_records=1000
data_availability_measurement_root=<da-root-hex>
data_availability_measurement_signature=<da-signature-hex>
data_availability_measurement=<receipt-root-hex>,available,<observed-block>
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
run_started_at_unix_seconds=<unix-seconds>
run_ended_at_unix_seconds=<unix-seconds-plus-at-least-604800>
run_window_signature=<window-signature-hex>
observed_blocks=100800
finalized_blocks=100800
checked_receipts=1000
available_receipts=1000
invalid_receipts_submitted=1
invalid_receipts_rejected=1
invalid_work_rejection_records=1
invalid_work_rejection_root=<invalid-work-root-hex>
invalid_work_rejection_signature=<invalid-work-signature-hex>
invalid_work_rejection=<receipt-root-hex>,rejected,<observed-block>
reward_settlement_records=1
reward_settlement_root=<reward-settlement-root-hex>
reward_settlement_signature=<reward-settlement-signature-hex>
reward_settlement=<receipt-root-hex>,<miner-id-hex>,<validator-id-hex>,<observed-block>
detection_measurement_records=1
detection_measurement_root=<detection-root-hex>
detection_measurement_signature=<detection-signature-hex>
detection_measurement=<mechanism>,<subject-root-hex>,<sample-count>,<detected-count>,<observed-block>
cuda_verified_miner_count=<counted-public-miners>
cuda_graph_execution_receipts=<cuda-graph-receipt-count>
validator_vrf_lifecycle_records=<checked-receipt-count>
validator_vrf_lifecycle_root=<validator-vrf-lifecycle-root-hex>
validator_vrf_lifecycle_signature=<validator-vrf-lifecycle-signature-hex>
validator_vrf_lifecycle=<receipt-root-hex>,<validator-id-hex>,<beacon-round>,revealed,<observed-block>
node=miner,<address-hex>,<operator-id-hex>,0,100799,<heartbeat-count>,<heartbeat-signature-hex>
node=validator,<address-hex>,<operator-id-hex>,0,100799,<heartbeat-count>,<heartbeat-signature-hex>
service=rpc,<endpoint-id-hex>,https://rpc.tensorvm.net/health,/health,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
service=explorer,<endpoint-id-hex>,https://explorer.tensorvm.net/health,/health,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
service=faucet,<endpoint-id-hex>,https://faucet.tensorvm.net/health,/health,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
service=telemetry,<endpoint-id-hex>,https://telemetry.tensorvm.net/health,/health,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
service_content=rpc,<endpoint-id-hex>,https://rpc.tensorvm.net/chain/head,/chain/head,<content-root-hex>,<unix-seconds>,<min-content-bytes>,<content-signature-hex>
service_content=explorer,<endpoint-id-hex>,https://explorer.tensorvm.net/explorer,/explorer,<content-root-hex>,<unix-seconds>,<min-content-bytes>,<content-signature-hex>
service_content=faucet,<endpoint-id-hex>,https://faucet.tensorvm.net/faucet/page,/faucet/page,<content-root-hex>,<unix-seconds>,<min-content-bytes>,<content-signature-hex>
service_content=telemetry,<endpoint-id-hex>,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,<content-root-hex>,<unix-seconds>,<min-content-bytes>,<content-signature-hex>
```

The CLI reads a manifest file and reports the default full-spec evidence status:

```bash
tvmd public evidence validate docs/tensorvm/public-testnet.evidence
tvmd public evidence validate deploy/tensorvm/manifests/public-testnet.evidence.example
```

Operators can generate the signed publication, run-window, node-heartbeat, and operator-attestation
manifest fields:

```bash
tvmd public evidence publish \
  --bundle-id <bundle-id-hex> \
  --public-uri https://tensorvm.net/tensorvm/public-evidence.json \
  --manifest-signer <manifest-signer-address-hex> \
  --manifest-signature-count 1 \
  --independent-auditor-count 1

tvmd public evidence audit \
  --bundle-id <bundle-id-hex> \
  --public-uri https://tensorvm.net/tensorvm/public-evidence.json \
  --auditor-id <auditor-address-hex> \
  --audit-uri https://auditor.tensorvm.net/tensorvm/audit.json \
  --observed-at <unix-seconds>

tvmd public evidence run window \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --started-at <unix-seconds> \
  --ended-at <unix-seconds-plus-at-least-604800> \
  --observed-blocks 100800

tvmd public evidence run window-file \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --block-observation-file artifacts/block-observations.records

tvmd public evidence node heartbeat \
  --role miner \
  --address <node-address-hex> \
  --operator-id <operator-id-hex> \
  --first-block 0 \
  --last-block 100799 \
  --heartbeat-count 100800

tvmd public evidence node heartbeat-file \
  --role miner \
  --address <node-address-hex> \
  --operator-id <operator-id-hex> \
  --heartbeat-file artifacts/miner-a-heartbeats.records

tvmd public evidence node operator-attestation \
  --role miner \
  --address <node-address-hex> \
  --operator-id <operator-id-hex> \
  --identity-uri https://operator-a.tensorvm.net/tensorvm.json \
  --observed-at <unix-seconds>
```

The publication command rejects non-public or malformed evidence URIs, HTTPS evidence URIs without a
concrete query-free path, zero bundle IDs, zero manifest signers, manifest signature counts other than `1`, and
zero auditor counts. The
auditor-record command rejects zero bundle IDs, non-public or malformed public or audit URIs, zero auditor
IDs, and empty observation times; bundle validation only counts auditor records whose auditor ID differs
from the manifest signer and whose observation timestamp is at or after the signed run-window end, and the
valid signed auditor-record count must exactly match `independent_auditor_count`. Its output can be
inserted directly as an `auditor=...` line in the evidence manifest. The run-window command
rejects zero
IDs/signers, inverted time windows, and empty block counts. The node-heartbeat command rejects zero node
addresses, zero operator IDs, inverted block ranges, and unsigned heartbeat summaries. Bundle validation
only counts a node toward the public run when its signed heartbeat count covers the manifest's observed
block count, and miner/validator operator IDs and addresses must be disjoint under the criteria-aware
one-to-one matching for the role minima to count independently.
The `evidence run window-file` form derives the same run-window manifest fields from raw
`run_window_observation=<block>,<unix-seconds>` records. It ignores blank lines and `#` comments, rejects
duplicate or non-contiguous block observations, zero timestamps, decreasing timestamps, unsupported lines,
and whitespace-padded records, then derives the signed start time, end time, and observed block count.
The `evidence node heartbeat-file` form derives the same signed `node=...` line from raw
`node_heartbeat_observation=<role>,<node-address-hex>,<operator-id-hex>,<block>` records. It ignores blank
lines and `#` comments, rejects duplicate or non-contiguous block observations, identity mismatches,
unsupported lines, and whitespace-padded records, then derives the signed first block, last block, and
heartbeat count.
The operator-attestation command rejects zero node addresses, zero operator IDs, non-public or malformed
identity URIs, and empty observation times; bundle validation only counts operator attestations observed
inside the signed run window and matching signed live node-heartbeat records. The
`operator_identity_attestation_records` count must exactly match those valid signed operator records and
the repeated `operator=...` line count. Its output can be inserted directly as an `operator=...` line in the
evidence manifest.

Operators can generate signed service-health and service-content manifest lines for RPC, explorer, faucet,
or telemetry evidence:

```bash
tvmd public evidence service health \
  --kind rpc \
  --endpoint-id <endpoint-id-hex> \
  --public-url https://rpc.tensorvm.net/health \
  --health-path /health \
  --first-block 0 \
  --last-block 100799 \
  --reachable-count 100800 \
  --signed-health-check-count 100800

tvmd public evidence service health-file \
  --kind rpc \
  --endpoint-id <endpoint-id-hex> \
  --public-url https://rpc.tensorvm.net/health \
  --health-path /health \
  --observation-file artifacts/rpc-health.records

tvmd public evidence service content \
  --kind rpc \
  --endpoint-id <endpoint-id-hex> \
  --public-url https://rpc.tensorvm.net/chain/head \
  --content-path /chain/head \
  --content-root <content-root-hex> \
  --observed-at <unix-seconds> \
  --min-content-bytes 64

tvmd public evidence service content-bytes \
  --kind rpc \
  --endpoint-id <endpoint-id-hex> \
  --public-url https://rpc.tensorvm.net/chain/head \
  --content-path /chain/head \
  --observed-at <unix-seconds> \
  --content-hex <captured-response-body-hex>

tvmd public evidence service content-file \
  --kind rpc \
  --endpoint-id <endpoint-id-hex> \
  --public-url https://rpc.tensorvm.net/chain/head \
  --content-path /chain/head \
  --observed-at <unix-seconds> \
  --content-file artifacts/rpc-chain-head.body
```

The command rejects non-public service URLs, health URLs whose path does not exactly match the signed
health path, health URLs with query strings or fragments, malformed endpoint IDs, invalid block ranges,
and unsigned or unreachable service-health summaries.
The `evidence service health-file` form derives the same signed `service=...` line from raw
`service_health_observation=<block>,reachable` and
`service_health_observation=<block>,unreachable` records. It ignores blank lines and `#` comments, rejects
duplicate or non-contiguous block observations, and rejects unsupported or whitespace-padded records before
deriving the signed first block, last block, reachable count, and signed health-check count.
Bundle validation only counts a service as deployed when both reachable observations and signed health
checks cover the manifest's observed block count and the reachable count does not exceed the signed
health-check count. Its output can be inserted directly as a `service=...` line in the evidence manifest.
The service-content command rejects non-public content URLs, malformed
endpoint IDs, content URLs whose path does not exactly match the required service surface, content URLs
with query strings or fragments, zero content roots, empty observation times, and content proofs smaller
than 64 observed bytes.
`evidence service content-bytes` and `evidence service content-file` are the reproducible paths from a captured
response body to the same manifest line: they hash the exact bytes with the TensorVM service-content-root
domain, set `min_content_bytes` to the captured byte length, and reject malformed hex, unreadable files, or
captured bodies shorter than 64 bytes.
Bundle validation only counts service-content records observed inside the signed run window and whose HTTPS
authority matches the corresponding service-health URL for the same endpoint ID. Its output can be inserted
directly as a `service_content=...` line in the evidence manifest. The public service gate requires both
lines for every RPC, explorer, faucet, and telemetry endpoint, with matching endpoint IDs and matching
HTTPS authorities.

Operators can also generate signed production libp2p runtime observation records before rolling them into
the required network-runtime summary root:

```bash
tvmd public evidence network observation \
  --operator-id <operator-id-hex> \
  --peer-id <libp2p-peer-id> \
  --listen-address /dns/node-a.tensorvm.net/tcp/4001 \
  --observed-at <unix-seconds> \
  --gossip-topics 5 \
  --request-response-protocols 4 \
  --bootstrap-peers 2 \
  --max-transmit-bytes 1048576 \
  --request-timeout-seconds 10 \
  --max-concurrent-streams 128 \
  --idle-timeout-seconds 60

tvmd public evidence network from-service-log \
  --operator-id <operator-id-hex> \
  --listen-address /dns/node-a.tensorvm.net/tcp/4001 \
  --observed-at <unix-seconds> \
  --service-log artifacts/node-a-tvmd-service.log
```

The command rejects zero operator IDs, malformed peer IDs, malformed libp2p multiaddrs, listen multiaddrs
without a nonzero TCP port, malformed DNS labels, single-label DNS hosts, and listen multiaddrs using
localhost, `.local`, special-use DNS names, loopback, unspecified, private, link-local, documentation,
shared-address, benchmarking, multicast, or reserved IP hosts. It also rejects missing discovery/bootstrap
observations, missing gossip or request-response protocol counts, and missing DoS-control limits. Its
output is a signed `network_runtime_observation=...` line. Full-spec evidence must include one such line
per counted public miner or validator operator and must derive the `network-runtime` summary with
`evidence record summary-roots` over those observation roots.
The `evidence network from-service-log` form derives the peer ID, protocol counts, bootstrap-peer count,
and DoS-control limits from an exact captured `tvmd node serve` log, while still requiring the supplied
listen multiaddr to be public. It rejects logs that do not show `command=service_serve` and
`p2p_runtime=libp2p`, duplicate log fields, and missing runtime fields.

Operators can also generate signed supporting-record summary lines, including the production libp2p
network-observation summary required by full-spec evidence:

```bash
tvmd public evidence record summary \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --record-root <network-runtime-root-hex> \
  --record-count <operator-count>

tvmd public evidence record artifact \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --artifact-uri https://evidence.tensorvm.net/tensorvm/network-runtime.json \
  --record-root <network-runtime-root-hex> \
  --record-count <operator-count>

tvmd public evidence record artifact-roots \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --artifact-uri https://evidence.tensorvm.net/tensorvm/network-runtime.json \
  --record-roots <comma-separated-record-roots>

tvmd public evidence record artifact-file \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --artifact-uri https://evidence.tensorvm.net/tensorvm/network-runtime.json \
  --record-file artifacts/network-runtime.records

tvmd public evidence record summary-roots \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --record-roots <comma-separated-record-roots>

tvmd public evidence record summary-file \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --record-file artifacts/network-runtime.records
```

Supported record kinds are `block-history`, `finality-history`, `network-runtime`, `randomness-beacon`,
`data-availability`, `invalid-work`, `reward-settlement`, `detection-measurement`, and
`validator-vrf-lifecycle`. The command emits the corresponding `<record>_records`,
`<record>_root`, and `<record>_signature` manifest fields using the same signature domain the validator
checks.
The `evidence record artifact` command emits a signed `record_artifact=...` manifest line that binds an external
raw-record artifact URI to the same record kind, root, and count. The full independently checkable gate
requires one valid artifact locator for every required supporting-record summary root, distinct artifact
URIs across those required record kinds, and exactly nine supporting artifact locators total: block history,
finality history, network runtime, randomness beacon, data availability, invalid work, reward settlement,
detection measurement, and validator VRF lifecycle.
The `evidence record summary-roots` and `evidence record artifact-roots` variants derive a deterministic aggregate
root and record count from unique provided supporting-record roots before signing the summary fields or
artifact locator; duplicate roots are rejected so a summary count cannot be padded by repeating the same raw
record root. Empty or whitespace-padded comma-separated root entries are also rejected.
The `evidence record summary-file` and `evidence record artifact-file` variants derive those same fields from a
saved line-oriented raw-record file. Blank lines and `#` comments are ignored; generic raw-record files use
`record_root=<hex>` lines, and network-runtime files can contain the exact signed
`network_runtime_observation=...` lines emitted by the network-observation commands. File-derived
network-runtime roots verify the full signed observation line, including the libp2p peer ID, public
listen multiaddr, nonzero discovery/gossip/request-response/DoS-control counters, observation root, and
observation signature, before the root can be aggregated; full bundle validation also rejects counted
operators that reuse the same peer ID or public listen multiaddr. Non-network supporting-record files can contain
exact `block_history_record=...`, `finality_history_record=...`, `randomness_beacon_record=...`,
`data_availability_measurement=...`, `invalid_work_rejection=...`, `reward_settlement=...`,
`detection_measurement=...`, or `validator_vrf_lifecycle=...` raw record lines. These typed lines are hashed
with the record kind and exact line bytes only after the file parser validates the selected kind's fields:
`block_history_record=<block>,<block-root-hex>`,
`finality_history_record=<block>,<block-root-hex>,finalized|unfinalized`,
`randomness_beacon_record=<source-id-hex>,<round>,<randomness-root-hex>,<proof-root-hex>,drand-v1|validator-vrf-v1|local-deterministic-fixture-v1,<observed-block>,accepted|rejected`,
`data_availability_measurement=<receipt-root-hex>,available|unavailable,<block>`,
`invalid_work_rejection=<receipt-root-hex>,rejected,<block>`,
`reward_settlement=<receipt-root-hex>,<miner-id-hex>,<validator-id-hex>,<block>`,
`detection_measurement=<mechanism>,<subject-root-hex>,<sample-count>,<detected-count>,<block>`, and
`validator_vrf_lifecycle=<receipt-root-hex>,<validator-id-hex>,<beacon-round>,committed|revealed,<block>`.
Reward-settlement participant IDs must be valid 64-character hex IDs, detection mechanisms must be
lowercase ASCII letters, digits, or `-`, sample count must be nonzero, and detected count cannot exceed
sample count. Detection measurement subject roots must be nonzero. Saved raw artifacts can therefore produce matching
summary roots and artifact locators without hand-copying individual `record_root=<hex>` values.
Whitespace-padded record lines and empty fields are rejected.

The output is a line-oriented evidence report. `public_evidence_full_spec=true` requires the default
public-testnet criteria or stricter criteria, `public_criterion=true`, `independently_checkable=true`,
`cuda_verified_miner_count` covering the counted public miners, positive `cuda_graph_execution_receipts`
that do not exceed checked or available receipt counts and are backed by that CUDA miner coverage,
signed `validator_vrf_lifecycle_records` with raw `validator_vrf_lifecycle=...` lines covering every
checked and available receipt's deployed commit-to-reveal lifecycle, requiring `available_receipts` to
equal `checked_receipts`, using distinct nonzero receipt roots, and
aggregating to the signed lifecycle root,
positive signed deployed `detection_measurement_records`,
the signed randomness-beacon summary count to equal `observed_blocks`,
manifest-level raw accepted public `randomness_beacon_record=...` lines for every observed block with
distinct source/round pairs, and
manifest-level raw `block_history_record=...` and `finality_history_record=...` lines with distinct
nonzero block roots, matching block numbers/roots, and finalized status counts matching `finalized_blocks`,
`data_availability_measurement=...` lines with distinct nonzero receipt roots,
`invalid_work_rejection=...` lines with distinct nonzero receipt roots,
`reward_settlement=...` lines with distinct nonzero receipt roots and nonzero miner/validator IDs, and
`detection_measurement=...` lines with valid mechanism labels, nonzero subject roots, nonzero samples,
and detected counts not exceeding samples, plus raw `validator_vrf_lifecycle=...` lines with distinct receipt roots whose aggregate roots
match the signed chain-history and operational summaries. Full-spec randomness
records must be `accepted` and use `drand-v1` or `validator-vrf-v1`; a
`local-deterministic-fixture-v1` record can exercise parsers but cannot satisfy full-spec public randomness
evidence.
Relaxed local harness criteria can exercise the validator but cannot set the full-spec flag. The
`external_operator_evidence` field is true only when enough signed node evidence and matching signed
operator identity attestation records are present.
The individual fields identify which post-run artifact or protocol observation is missing:

```text
public_evidence_full_spec=false
public_criterion=false
independently_checkable=true
published_evidence_bundle=true
independent_auditor_records=true
signed_run_window=true
block_history=true
finality_history=true
operator_identity_attestations=true
network_runtime_observations=true
data_availability_measurements=true
signed_invalid_work_rejection_records=true
signed_reward_settlement_records=true
signed_validator_vrf_lifecycle_records=true
supporting_record_artifacts=true
cuda_verified_miners=true
cuda_graph_execution_evidence=true
miners=2
validators=1
run_started_at_unix_seconds=1700000000
run_ended_at_unix_seconds=1700000060
observed_duration_seconds=60
required_duration_seconds=604800
observed_blocks=10
required_blocks=100800
finality_rate_bps=10000
data_availability_bps=9500
invalid_receipts_submitted=1
invalid_receipts_rejected=1
invalid_work_rejection_rate_bps=10000
reward_settlement_records=1
cuda_verified_miner_count=2
cuda_graph_execution_receipts=1
validator_vrf_lifecycle_records=20
external_operator_evidence=true
required_miners=false
required_validators=false
required_run_duration=false
required_block_count=false
required_finality=true
required_data_availability=true
invalid_work_rejection_evidence=true
reward_settlement_evidence=true
cuda_miner_evidence=true
cuda_graph_execution_receipt_evidence=true
validator_vrf_lifecycle_record_evidence=true
production_libp2p_runtime=true
deployed_rpc_service=true
deployed_explorer_service=true
deployed_faucet_service=true
deployed_telemetry_service=true
deployed_public_service_content=true
deployed_public_services=true
```
