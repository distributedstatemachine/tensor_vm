#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BUNDLE_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(CDPATH= cd -- "$BUNDLE_DIR/../../.." && pwd)
COMPOSE_FILE="$BUNDLE_DIR/docker-compose.yml"
RPC_PORT="${TENSORVM_LOCAL_CPU_RPC_PORT:-8545}"
EXPLORER_PORT="${TENSORVM_LOCAL_CPU_EXPLORER_PORT:-8080}"
AUTH_TOKEN="${TENSORVM_AUTH_TOKEN:-local-cpu-testnet-token}"
EXPECTED_RANDOMNESS_BEACON_MODE="${TENSORVM_RANDOMNESS_BEACON_MODE:-local_deterministic}"
EXPECTED_RANDOMNESS_BEACON_SOURCE_ID="${TENSORVM_RANDOMNESS_BEACON_SOURCE_ID:-local_drand_fixture_v1}"
EXPECTED_RANDOMNESS_BEACON_ROUND="${TENSORVM_RANDOMNESS_BEACON_ROUND:-1000}"
EXPECTED_RANDOMNESS_BEACON_SOURCE_LABEL="${EXPECTED_RANDOMNESS_BEACON_MODE}:${EXPECTED_RANDOMNESS_BEACON_SOURCE_ID}"
TOPOLOGY_FILE="$SCRIPT_DIR/local-cpu-topology.sh"

fail() {
  echo "local CPU testnet check failed: $*" >&2
  exit 1
}

debug() {
  if [ "${TENSORVM_LOCAL_CPU_CHECK_DEBUG:-false}" = "true" ]; then
    echo "local CPU testnet check debug: $*" >&2
  fi
}

[ -r "$TOPOLOGY_FILE" ] || fail "local CPU topology file is not readable"
. "$TOPOLOGY_FILE"
EXPECTED_SERVICES="$LOCAL_CPU_EXPECTED_SERVICES"
MINERS="$LOCAL_CPU_MINERS"
VALIDATORS="$LOCAL_CPU_VALIDATORS"
EXPECTED_SERVICE_COUNT="$LOCAL_CPU_EXPECTED_SERVICE_COUNT"
EXPECTED_MINER_COUNT="$LOCAL_CPU_MINER_COUNT"
EXPECTED_VALIDATOR_COUNT="$LOCAL_CPU_VALIDATOR_COUNT"
EXPECTED_SETTLED_RECEIPTS="$LOCAL_CPU_EXPECTED_SETTLED_RECEIPTS"
EXPECTED_CUDA_REQUIRED_MINER_COUNT="$LOCAL_CPU_CUDA_REQUIRED_MINER_COUNT"
EXPECTED_BOOTSTRAP_SERVICE="$LOCAL_CPU_BOOTSTRAP_SERVICE"
EXPECTED_NETWORK_OBSERVER_SERVICE="$LOCAL_CPU_NETWORK_OBSERVER_SERVICE"
EXPECTED_SEED_HEIGHT="$LOCAL_CPU_SEED_HEIGHT"
EXPECTED_SEED_BLOCKS="$LOCAL_CPU_SEED_BLOCKS"
EXPECTED_FULL_RATE_BPS="$LOCAL_CPU_FULL_RATE_BPS"
EXPECTED_LIVE_PRIMITIVE_RECEIPT_FLOOR="$LOCAL_CPU_LIVE_PRIMITIVE_RECEIPT_FLOOR"
EXPECTED_LIVE_RECEIPT_QUERY_LIMIT="$LOCAL_CPU_LIVE_RECEIPT_QUERY_LIMIT"
EXPECTED_BLOCK_SCAN_DEPTH="$LOCAL_CPU_BLOCK_SCAN_DEPTH"
EXPECTED_CHECKER_RETRY_LIMIT="$LOCAL_CPU_CHECKER_RETRY_LIMIT"
EXPECTED_OPERATOR_CONVERGENCE_RETRY_LIMIT="$LOCAL_CPU_OPERATOR_CONVERGENCE_RETRY_LIMIT"
EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS="$LOCAL_CPU_DOCKER_EXEC_TIMEOUT_SECONDS"
EXPECTED_HTTP_TIMEOUT_SECONDS="$LOCAL_CPU_HTTP_TIMEOUT_SECONDS"
EXPECTED_CHECKER_RETRY_SLEEP_SECONDS="$LOCAL_CPU_CHECKER_RETRY_SLEEP_SECONDS"
RESTART_CONTINUITY_MODE="${TENSORVM_LOCAL_CPU_RESTART_CONTINUITY_MODE:-false}"
RESTART_CONTINUITY_SERVICES="${TENSORVM_LOCAL_CPU_RESTART_SERVICES:-}"

compose() {
  docker compose -f "$COMPOSE_FILE" "$@" < /dev/null
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

contains_line() {
  case "
$1
" in
    *"
$2
"*) return 0 ;;
    *) return 1 ;;
  esac
}

text_contains() {
  case "$1" in
    *"$2"*) return 0 ;;
    *) return 1 ;;
  esac
}

csv_contains_value() {
  case ",$1," in
    *",$2,"*) return 0 ;;
    *) return 1 ;;
  esac
}

word_list_contains() {
  case " $1 " in
    *" $2 "*) return 0 ;;
    *) return 1 ;;
  esac
}

restart_plateau_service_ready() {
  [ "$RESTART_CONTINUITY_MODE" = "true" ] \
    && word_list_contains "$RESTART_CONTINUITY_SERVICES" "$service" \
    && [ "$SERVICE_HEIGHT" -gt "$EXPECTED_SEED_HEIGHT" ] \
    && [ "$SERVICE_BLOCK_COUNT" -gt "$EXPECTED_SEED_BLOCKS" ] \
    && [ "$SERVICE_FINALIZED_BLOCK_COUNT" -gt "$EXPECTED_SEED_BLOCKS" ] \
    && [ "$SERVICE_STATE_ROOT" != "$ZERO_HASH" ] \
    && [ "$SERVICE_BLOCK_LOG_ROOT" != "$ZERO_HASH" ] \
    && [ "$SERVICE_ROLE_P2P_CONNECTED_PEERS" -gt 0 ]
}

unique_count() {
  sort -u "$1" | wc -l | tr -d ' '
}

read_service_file() {
  service="$1"
  path="$2"
  output=$(compose exec -T "$service" cat "$path") || return 1
  printf '%s\n' "$output" | tr -d '\r'
}

read_ready_report() {
  read_service_file "$1" /var/lib/tensorvm/local-cpu-ready
}

read_seed_report() {
  read_service_file "$1" /var/lib/tensorvm/local-testnet-seed.out
}

status_value() {
  key="$1"
  document="$2"
  prefix="${key}="
  while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in
      "$prefix"*)
        printf '%s\n' "${line#"$prefix"}"
        return 0
        ;;
    esac
  done <<EOF
$document
EOF
  printf '\n'
}

is_u64() {
  case "$1" in
    ""|*[!0-9]*) return 1 ;;
    *) return 0 ;;
  esac
}

require_command docker
require_command sort
require_command wc
require_command curl
require_command python3
require_command timeout

json_bool_true() {
  key="$1"
  document="$2"
  printf '%s\n' "$document" | python3 -c '
import json
import sys

try:
    value = json.load(sys.stdin)[sys.argv[1]]
except (KeyError, TypeError, json.JSONDecodeError):
    sys.exit(1)
sys.exit(0 if value is True else 1)
' "$key"
}

json_number() {
  key="$1"
  document="$2"
  printf '%s\n' "$document" | python3 -c '
import json
import sys

try:
    value = json.load(sys.stdin)[sys.argv[1]]
except (KeyError, TypeError, json.JSONDecodeError):
    sys.exit(1)
if isinstance(value, int) and not isinstance(value, bool) and value >= 0:
    print(value)
    sys.exit(0)
sys.exit(1)
' "$key"
}

json_summary_number() {
  key="$1"
  document="$2"
  printf '%s\n' "$document" | python3 -c '
import json
import sys

try:
    value = json.load(sys.stdin)["summary"][sys.argv[1]]
except (KeyError, TypeError, json.JSONDecodeError):
    sys.exit(1)
if isinstance(value, int) and not isinstance(value, bool) and value >= 0:
    print(value)
    sys.exit(0)
sys.exit(1)
	' "$key"
}

json_randomness_number() {
  key="$1"
  document="$2"
  printf '%s\n' "$document" | python3 -c '
import json
import sys

try:
    value = json.load(sys.stdin)["randomness_binding_evidence"][sys.argv[1]]
except (KeyError, TypeError, json.JSONDecodeError):
    sys.exit(1)
if isinstance(value, int) and not isinstance(value, bool) and value >= 0:
    print(value)
    sys.exit(0)
sys.exit(1)
' "$key"
}

json_randomness_bool() {
  key="$1"
  document="$2"
  printf '%s\n' "$document" | python3 -c '
import json
import sys

try:
    value = json.load(sys.stdin)["randomness_binding_evidence"][sys.argv[1]]
except (KeyError, TypeError, json.JSONDecodeError):
    sys.exit(1)
print("true" if value is True else "false" if value is False else "")
sys.exit(0 if isinstance(value, bool) else 1)
' "$key"
}

json_string() {
  key="$1"
  document="$2"
  printf '%s\n' "$document" | python3 -c '
import json
import sys

try:
    value = json.load(sys.stdin)[sys.argv[1]]
except (KeyError, TypeError, json.JSONDecodeError):
    sys.exit(1)
if isinstance(value, str):
    print(value)
    sys.exit(0)
sys.exit(1)
' "$key"
}

json_array_length() {
  key="$1"
  document="$2"
  printf '%s\n' "$document" | python3 -c '
import json
import sys

try:
    value = json.load(sys.stdin)[sys.argv[1]]
except (KeyError, TypeError, json.JSONDecodeError):
    sys.exit(1)
if isinstance(value, list):
    print(len(value))
    sys.exit(0)
sys.exit(1)
' "$key"
}

json_positive_field_count() {
  key="$1"
  document="$2"
  printf '%s\n' "$document" | python3 -c '
import json
import sys

def values(value):
    if isinstance(value, dict):
        yield value
        for nested in value.values():
            yield from values(nested)
    elif isinstance(value, list):
        for nested in value:
            yield from values(nested)

try:
    document = json.load(sys.stdin)
except json.JSONDecodeError:
    sys.exit(1)
key = sys.argv[1]
count = 0
for item in values(document):
    value = item.get(key)
    if isinstance(value, int) and not isinstance(value, bool) and value > 0:
        count += 1
print(count)
' "$key"
}

json_string_field_count() {
  key="$1"
  value="$2"
  document="$3"
  printf '%s\n' "$document" | python3 -c '
import json
import sys

def values(value):
    if isinstance(value, dict):
        yield value
        for nested in value.values():
            yield from values(nested)
    elif isinstance(value, list):
        for nested in value:
            yield from values(nested)

try:
    document = json.load(sys.stdin)
except json.JSONDecodeError:
    sys.exit(1)
key = sys.argv[1]
expected = sys.argv[2]
count = 0
for item in values(document):
    if item.get(key) == expected:
        count += 1
print(count)
' "$key" "$value"
}

json_future_pending_reward_count() {
  ledger="$1"
  min_height="$2"
  document="$3"
  printf '%s\n' "$document" | python3 -c '
import json
import sys

try:
    document = json.load(sys.stdin)
except json.JSONDecodeError:
    sys.exit(1)
ledger = sys.argv[1]
try:
    min_height = int(sys.argv[2])
except ValueError:
    sys.exit(1)
rewards = document.get("pending_rewards")
if not isinstance(rewards, list):
    sys.exit(1)
count = 0
for reward in rewards:
    if not isinstance(reward, dict):
        continue
    amount = reward.get("amount")
    claimable_at_height = reward.get("claimable_at_height")
    reward_ledger = reward.get("ledger")
    if ledger == "receipt":
        if reward_ledger not in ("receipt_miner", "receipt_validator"):
            continue
    elif reward_ledger != ledger:
        continue
    if not isinstance(amount, int) or isinstance(amount, bool) or amount <= 0:
        continue
    if not isinstance(claimable_at_height, int) or isinstance(claimable_at_height, bool):
        continue
    if claimable_at_height > min_height and (
        ledger == "proposer" or reward.get("voided_by_challenge") is not True
    ):
        count += 1
print(count)
' "$ledger" "$min_height"
}

read_service_status() {
  service="$1"
  attempt=0
  while [ "$attempt" -lt "$EXPECTED_CHECKER_RETRY_LIMIT" ]; do
    debug "status probe service=$service attempt=$attempt"
    if output=$(timeout "${EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node status --data-dir /var/lib/tensorvm 2>/dev/null < /dev/null); then
      output=$(printf '%s\n' "$output" | tr -d '\r')
      role_loop_ready=$(status_value role_loop_ready "$output")
      if [ -n "$role_loop_ready" ] && [ "$role_loop_ready" != "unknown" ]; then
        printf '%s\n' "$output"
        return 0
      fi
    fi
    attempt=$((attempt + 1))
    sleep "$EXPECTED_CHECKER_RETRY_SLEEP_SECONDS"
  done
  return 1
}

read_service_block() {
  service="$1"
  height="$2"
  attempt=0
  while [ "$attempt" -lt "$EXPECTED_CHECKER_RETRY_LIMIT" ]; do
    debug "block probe service=$service height=$height attempt=$attempt"
    if output=$(timeout "${EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node block --data-dir /var/lib/tensorvm --height "$height" 2>/dev/null < /dev/null); then
      printf '%s\n' "$output" | tr -d '\r'
      return 0
    fi
    attempt=$((attempt + 1))
    sleep "$EXPECTED_CHECKER_RETRY_SLEEP_SECONDS"
  done
  return 1
}

cd "$REPO_ROOT"

compose config --quiet

CONFIG_SERVICES=$(compose config --services)
RUNNING_SERVICES=$(compose ps --status running --services)

for service in $EXPECTED_SERVICES; do
  contains_line "$CONFIG_SERVICES" "$service" || fail "compose config is missing $service"
  contains_line "$RUNNING_SERVICES" "$service" || fail "$service is not running"
done
contains_line "$CONFIG_SERVICES" "explorer" || fail "compose config is missing standalone explorer"
contains_line "$RUNNING_SERVICES" "explorer" || fail "standalone explorer is not running"

TMP_DIR="${TMPDIR:-/tmp}/tensorvm-local-cpu-check.$$"
mkdir -p "$TMP_DIR"
trap 'rm -rf "$TMP_DIR"' EXIT INT TERM

ZERO_HASH="0000000000000000000000000000000000000000000000000000000000000000"

for service in $EXPECTED_SERVICES; do
  READY_REPORT=$(read_ready_report "$service") \
    || fail "$service has not written /var/lib/tensorvm/local-cpu-ready"
  [ "$(status_value operator_name "$READY_REPORT")" = "$service" ] \
    || fail "$service readiness file does not name its operator"
  [ "$(status_value p2p_runtime "$READY_REPORT")" = "libp2p" ] \
    || fail "$service is missing libp2p runtime readiness"
  [ "$(status_value node_store_ready "$READY_REPORT")" = "true" ] \
    || fail "$service is missing node store readiness"
  [ "$(status_value libp2p_ready "$READY_REPORT")" = "true" ] \
    || fail "$service is missing libp2p readiness"
  [ "$(status_value p2p_identity_seeded "$READY_REPORT")" = "true" ] \
    || fail "$service is missing stable libp2p identity readiness"
  operator_id=$(compose exec -T "$service" printenv TENSORVM_OPERATOR_ID)
  [ "$(status_value p2p_identity_seed "$READY_REPORT")" = "$operator_id" ] \
    || fail "$service libp2p identity seed does not match its operator ID"
  READY_LOCAL_CPU_SYNTHETIC_JOB_PRODUCER=$(status_value local_cpu_synthetic_job_producer "$READY_REPORT")
  READY_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER=$(status_value local_cpu_validator_block_proposer "$READY_REPORT")
  READY_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS=$(status_value local_cpu_validator_block_proposer_delay_blocks "$READY_REPORT")
  READY_LOCAL_CPU_PROPOSER_COOLDOWN_BLOCKS=$(status_value local_cpu_proposer_cooldown_blocks "$READY_REPORT")
  [ -n "$READY_LOCAL_CPU_SYNTHETIC_JOB_PRODUCER" ] \
    || fail "$service readiness file does not report local CPU synthetic job producer mode"
  [ -n "$READY_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER" ] \
    || fail "$service readiness file does not report local CPU validator block proposer mode"
  [ -n "$READY_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS" ] \
    || fail "$service readiness file does not report local CPU validator block proposer delay"
  is_u64 "$READY_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS" \
    || fail "$service readiness file reports a non-numeric local CPU validator block proposer delay"
  [ -n "$READY_LOCAL_CPU_PROPOSER_COOLDOWN_BLOCKS" ] \
    || fail "$service readiness file does not report local CPU proposer cooldown"
  is_u64 "$READY_LOCAL_CPU_PROPOSER_COOLDOWN_BLOCKS" \
    || fail "$service readiness file reports a non-numeric local CPU proposer cooldown"
  [ "$READY_LOCAL_CPU_PROPOSER_COOLDOWN_BLOCKS" -gt 0 ] \
    || fail "$service readiness file reports proposer cooldown disabled"
  [ "$(status_value chain_profile "$READY_REPORT")" = "local_cpu" ] \
    || fail "$service readiness file does not report the local CPU chain profile"
  READY_P2P_PEER_ID=$(status_value p2p_peer_id "$READY_REPORT")
  [ -n "$READY_P2P_PEER_ID" ] || fail "$service readiness file does not report a libp2p peer ID"
  READY_ROLE=$(status_value role "$READY_REPORT")
  READY_RUNTIME_COMMAND=$(status_value runtime_command "$READY_REPORT")
  case "$service" in
    miner-*)
      [ "$READY_ROLE" = "miner" ] || fail "$service is not marked as a miner"
      [ "$READY_RUNTIME_COMMAND" = "miner_run" ] || fail "$service is not running the miner role command"
      [ "$(status_value device "$READY_REPORT")" = "cpu" ] || fail "$service is not using the CPU backend"
      ;;
    validator-*)
      [ "$READY_ROLE" = "validator" ] || fail "$service is not marked as a validator"
      [ "$READY_RUNTIME_COMMAND" = "validator_run" ] || fail "$service is not running the validator role command"
      [ "$(status_value reference_verifier_ready "$READY_REPORT")" = "true" ] \
        || fail "$service validator readiness is missing"
      ;;
    *)
      fail "unexpected local CPU service role: $service"
      ;;
  esac
  printf '%s\n' "$READY_P2P_PEER_ID" >> "$TMP_DIR/p2p_peer_ids"
  printf '%s\n' "$operator_id" >> "$TMP_DIR/operator_ids"
  compose exec -T "$service" printenv TENSORVM_NODE_MULTIADDR >> "$TMP_DIR/node_multiaddrs"
done

[ "$(unique_count "$TMP_DIR/operator_ids")" = "$EXPECTED_SERVICE_COUNT" ] || fail "operator IDs are not distinct"
[ "$(unique_count "$TMP_DIR/p2p_peer_ids")" = "$EXPECTED_SERVICE_COUNT" ] || fail "libp2p peer IDs are not distinct"
[ "$(unique_count "$TMP_DIR/node_multiaddrs")" = "$EXPECTED_SERVICE_COUNT" ] || fail "node multiaddrs are not distinct"

for service in $EXPECTED_SERVICES; do
  SEED_REPORT=$(read_seed_report "$service") \
    || fail "$service did not seed local testnet chain state"
  [ "$(status_value command "$SEED_REPORT")" = "local_testnet_seed" ] \
    || fail "$service did not seed local testnet chain state"
  [ "$(status_value height "$SEED_REPORT")" = "$EXPECTED_SEED_HEIGHT" ] \
    || fail "$service seeded local testnet did not start at height $EXPECTED_SEED_HEIGHT"
  [ "$(status_value blocks "$SEED_REPORT")" = "$EXPECTED_SEED_BLOCKS" ] \
    || fail "$service seeded local testnet did not start with $EXPECTED_SEED_BLOCKS blocks"
  LOCAL_CPU_VERIFY=$(compose exec -T "$service" tvmd localnet verify --data-dir /var/lib/tensorvm --json | tr -d '\r')
  json_bool_true structured_verifier_ready "$LOCAL_CPU_VERIFY" \
    || fail "$service local CPU structured verifier is not ready"
  json_bool_true ready "$LOCAL_CPU_VERIFY" \
    || fail "$service local CPU structured verifier did not accept node store"
done

MINER_SEED_REPORT=$(read_seed_report "$EXPECTED_BOOTSTRAP_SERVICE") \
  || fail "$EXPECTED_BOOTSTRAP_SERVICE did not seed local testnet chain state"
[ "$(status_value command "$MINER_SEED_REPORT")" = "local_testnet_seed" ] \
  || fail "$EXPECTED_BOOTSTRAP_SERVICE did not seed local testnet chain state"
SEED_SETTLED_RECEIPTS=$(status_value settled_receipts "$MINER_SEED_REPORT")
[ "$SEED_SETTLED_RECEIPTS" = "$EXPECTED_SETTLED_RECEIPTS" ] \
  || fail "seeded local testnet did not report settled receipts"
SEED_MATMUL_SETTLED=$(status_value matmul_settled "$MINER_SEED_REPORT")
[ "$SEED_MATMUL_SETTLED" = "true" ] \
  || fail "seeded local testnet did not settle matmul work"
SEED_LINEAR_TRAINING_SETTLED=$(status_value linear_training_settled "$MINER_SEED_REPORT")
[ "$SEED_LINEAR_TRAINING_SETTLED" = "true" ] \
  || fail "seeded local testnet did not settle linear training work"
SEED_FINALITY_RATE_BPS=$(status_value finality_rate_bps "$MINER_SEED_REPORT")
[ "$SEED_FINALITY_RATE_BPS" = "$EXPECTED_FULL_RATE_BPS" ] \
  || fail "seeded local testnet did not report full finality"
SEED_DATA_AVAILABILITY_BPS=$(status_value data_availability_bps "$MINER_SEED_REPORT")
[ "$SEED_DATA_AVAILABILITY_BPS" = "$EXPECTED_FULL_RATE_BPS" ] \
  || fail "seeded local testnet did not report full data availability"
SEED_REWARDED_MINERS=$(status_value rewarded_miners "$MINER_SEED_REPORT")
[ "${SEED_REWARDED_MINERS:-0}" -gt 0 ] || fail "seeded local testnet did not report miner rewards"
SEED_PENDING_RECEIPT_REWARDS=$(status_value pending_receipt_rewards "$MINER_SEED_REPORT")
[ "${SEED_PENDING_RECEIPT_REWARDS:-0}" -gt 0 ] || fail "seeded local testnet did not report pending receipt rewards"
SEED_TOTAL_REWARD_BALANCE=$(status_value total_reward_balance "$MINER_SEED_REPORT")
[ -n "$SEED_TOTAL_REWARD_BALANCE" ] || fail "seeded local testnet did not report total reward balance"
SEED_ATTESTATION_COUNT=$(status_value attestation_count "$MINER_SEED_REPORT")
[ -n "$SEED_ATTESTATION_COUNT" ] || fail "seeded local testnet did not report attestation count"

for path in /health /rpc/health /chain/head /jobs/current /explorer/health /explorer /faucet/health /faucet/page /telemetry/health /telemetry/dashboard; do
  curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}${path}" >/dev/null \
    || fail "gateway route is not reachable: $path"
done

EXPLORER_HEALTH=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" "http://127.0.0.1:${EXPLORER_PORT}/health")
json_bool_true tensorvm_explorer_ready "$EXPLORER_HEALTH" \
  || fail "standalone explorer health is not ready"
EXPLORER_WS_URL=$(json_string websocket_url "$EXPLORER_HEALTH") \
  || fail "standalone explorer does not publish the TensorVM websocket URL"
text_contains "$EXPLORER_WS_URL" "/explorer/ws?token=" \
  || fail "standalone explorer does not publish the TensorVM websocket URL"
EXPLORER_PAGE=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" "http://127.0.0.1:${EXPLORER_PORT}/")
text_contains "$EXPLORER_PAGE" "TensorVM Explorer" \
  || fail "standalone explorer page is not reachable"
text_contains "$EXPLORER_PAGE" 'data-ui="ratzilla-tui"' \
  || fail "standalone explorer page is not the default Ratzilla-style TUI"
text_contains "$EXPLORER_PAGE" "new WebSocket" \
  || fail "standalone explorer page does not poll TensorVM over websocket"

LIVE_CHAIN_HEAD=""
LIVE_HEIGHT=0
LIVE_BLOCK_COUNT=0
LIVE_OVERVIEW=""
LIVE_FINALIZED_BLOCK_COUNT=0
LIVE_JOB_COUNT=0
LIVE_MODEL_COUNT=0
LIVE_ATTESTATION_COUNT=0
LIVE_RECEIPT_COUNT=0
LIVE_SETTLED_RECEIPT_COUNT=0
LIVE_PENDING_RECEIPT_REWARD_COUNT=0
LIVE_PENDING_CHALLENGE_REWARD_COUNT=0
LIVE_TOTAL_REWARD_BALANCE=0
LIVE_RECEIPTS=""
LIVE_ATTESTED_RECEIPT_COUNT=0
LIVE_TENSOR_OP_RECEIPT_COUNT=0
LIVE_LINEAR_TRAINING_RECEIPT_COUNT=0
LIVE_MODEL_STEP_TOTAL=0
LIVE_PENDING_PROPOSER_REWARD_COUNT=0
LIVE_DELAYED_RECEIPT_REWARD_CLAIMS=0
LIVE_DELAYED_PROPOSER_REWARD_CLAIMS=0
LIVE_DELAYED_CHALLENGE_REWARD_CLAIMS=0
LIVE_EXTERNAL_RANDOMNESS_BEACON_RECORDS=0
LIVE_VALIDATOR_VRF_REVEAL_COUNT=0
LIVE_LATEST_EXTERNAL_RANDOMNESS_BEACON_ROUND=0
LIVE_RANDOMNESS_CURRENT_BLOCK_HASH_ALLOWED=true
LIVE_RANDOMNESS_RECEIPT_ANCHORS_CONSISTENT=false
attempt=0
while [ "$attempt" -lt "$EXPECTED_CHECKER_RETRY_LIMIT" ]; do
  LIVE_CHAIN_HEAD=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/chain/head")
  LIVE_HEIGHT=$(json_number height "$LIVE_CHAIN_HEAD")
  LIVE_BLOCK_COUNT=$(json_number block_count "$LIVE_CHAIN_HEAD")
  LIVE_OVERVIEW=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/explorer/overview")
  LIVE_FINALIZED_BLOCK_COUNT=$(json_summary_number finalized_block_count "$LIVE_OVERVIEW")
  LIVE_JOB_COUNT=$(json_summary_number job_count "$LIVE_OVERVIEW")
  LIVE_MODEL_COUNT=$(json_summary_number model_count "$LIVE_OVERVIEW")
  LIVE_MODEL_STEP_TOTAL=$(json_summary_number model_step_total "$LIVE_OVERVIEW")
  LIVE_ATTESTATION_COUNT=$(json_summary_number attestation_count "$LIVE_OVERVIEW")
  LIVE_RECEIPT_COUNT=$(json_summary_number receipt_count "$LIVE_OVERVIEW")
  LIVE_SETTLED_RECEIPT_COUNT=$(json_summary_number settled_receipt_count "$LIVE_OVERVIEW")
  LIVE_PENDING_RECEIPT_REWARD_COUNT=$(json_summary_number pending_receipt_reward_count "$LIVE_OVERVIEW")
  LIVE_PENDING_PROPOSER_REWARD_COUNT=$(json_summary_number pending_proposer_reward_count "$LIVE_OVERVIEW")
  LIVE_PENDING_CHALLENGE_REWARD_COUNT=$(json_summary_number pending_challenge_reward_count "$LIVE_OVERVIEW")
  LIVE_TOTAL_REWARD_BALANCE=$(json_summary_number total_reward_balance "$LIVE_OVERVIEW")
  LIVE_DELAYED_RECEIPT_REWARD_CLAIMS=$(json_future_pending_reward_count receipt "$LIVE_HEIGHT" "$LIVE_OVERVIEW")
  LIVE_DELAYED_PROPOSER_REWARD_CLAIMS=$(json_future_pending_reward_count proposer "$LIVE_HEIGHT" "$LIVE_OVERVIEW")
  LIVE_DELAYED_CHALLENGE_REWARD_CLAIMS=$(json_future_pending_reward_count challenge "$LIVE_HEIGHT" "$LIVE_OVERVIEW")
  LIVE_EXTERNAL_RANDOMNESS_BEACON_RECORDS=$(json_randomness_number external_beacon_record_count "$LIVE_OVERVIEW")
  LIVE_VALIDATOR_VRF_REVEAL_COUNT=$(json_randomness_number validator_vrf_reveal_count "$LIVE_OVERVIEW")
  LIVE_LATEST_EXTERNAL_RANDOMNESS_BEACON_ROUND=$(json_randomness_number latest_external_beacon_round "$LIVE_OVERVIEW")
  LIVE_RANDOMNESS_CURRENT_BLOCK_HASH_ALLOWED=$(json_randomness_bool current_block_hash_randomness_allowed "$LIVE_OVERVIEW")
  LIVE_RANDOMNESS_RECEIPT_ANCHORS_CONSISTENT=$(json_randomness_bool all_receipt_anchors_consistent "$LIVE_OVERVIEW")
  LIVE_RECEIPTS=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/explorer/receipts/latest/${EXPECTED_LIVE_RECEIPT_QUERY_LIMIT}")
  LIVE_ATTESTED_RECEIPT_COUNT=$(json_positive_field_count attestation_count "$LIVE_RECEIPTS")
  LIVE_TENSOR_OP_RECEIPT_COUNT=$(json_string_field_count primitive_type tensor_op "$LIVE_RECEIPTS")
  LIVE_LINEAR_TRAINING_RECEIPT_COUNT=$(json_string_field_count primitive_type linear_training_step "$LIVE_RECEIPTS")
  if [ "${LIVE_HEIGHT:-0}" -gt "$EXPECTED_SEED_HEIGHT" ] \
    && [ "${LIVE_BLOCK_COUNT:-0}" -gt "$EXPECTED_SEED_BLOCKS" ] \
    && [ "${LIVE_JOB_COUNT:-0}" -gt "$EXPECTED_SEED_HEIGHT" ] \
    && [ "${LIVE_MODEL_COUNT:-0}" -gt 0 ] \
    && [ "${LIVE_MODEL_STEP_TOTAL:-0}" -gt 0 ] \
    && [ "${LIVE_ATTESTATION_COUNT:-0}" -gt "$SEED_ATTESTATION_COUNT" ] \
    && [ "${LIVE_RECEIPT_COUNT:-0}" -gt "$EXPECTED_SETTLED_RECEIPTS" ] \
    && [ "${LIVE_SETTLED_RECEIPT_COUNT:-0}" -gt "$EXPECTED_SETTLED_RECEIPTS" ] \
    && [ "${LIVE_ATTESTED_RECEIPT_COUNT:-0}" -gt "$EXPECTED_SETTLED_RECEIPTS" ] \
    && [ "${LIVE_TENSOR_OP_RECEIPT_COUNT:-0}" -gt "$EXPECTED_LIVE_PRIMITIVE_RECEIPT_FLOOR" ] \
    && [ "${LIVE_LINEAR_TRAINING_RECEIPT_COUNT:-0}" -gt "$EXPECTED_LIVE_PRIMITIVE_RECEIPT_FLOOR" ] \
    && [ "${LIVE_PENDING_RECEIPT_REWARD_COUNT:-0}" -gt "$SEED_PENDING_RECEIPT_REWARDS" ] \
    && [ "${LIVE_PENDING_PROPOSER_REWARD_COUNT:-0}" -gt 0 ] \
    && [ "${LIVE_DELAYED_RECEIPT_REWARD_CLAIMS:-0}" -gt 0 ] \
    && [ "${LIVE_DELAYED_PROPOSER_REWARD_CLAIMS:-0}" -gt 0 ] \
    && [ "${LIVE_EXTERNAL_RANDOMNESS_BEACON_RECORDS:-0}" -gt 0 ] \
    && [ "${LIVE_VALIDATOR_VRF_REVEAL_COUNT:-0}" -gt 0 ] \
    && [ "${LIVE_LATEST_EXTERNAL_RANDOMNESS_BEACON_ROUND:-0}" -ge "$EXPECTED_RANDOMNESS_BEACON_ROUND" ] \
    && [ "$LIVE_RANDOMNESS_CURRENT_BLOCK_HASH_ALLOWED" = "false" ] \
    && [ "$LIVE_RANDOMNESS_RECEIPT_ANCHORS_CONSISTENT" = "true" ]; then
    break
  fi
  attempt=$((attempt + 1))
  sleep "$EXPECTED_CHECKER_RETRY_SLEEP_SECONDS"
done

[ "${LIVE_HEIGHT:-0}" -gt "$EXPECTED_SEED_HEIGHT" ] || fail "gateway chain head did not advance past seeded height $EXPECTED_SEED_HEIGHT"
[ "${LIVE_BLOCK_COUNT:-0}" -gt "$EXPECTED_SEED_BLOCKS" ] || fail "gateway chain block count did not advance past seeded $EXPECTED_SEED_BLOCKS blocks"
[ "${LIVE_FINALIZED_BLOCK_COUNT:-0}" -gt "$EXPECTED_SEED_BLOCKS" ] || fail "gateway finalized block count did not advance past seeded $EXPECTED_SEED_BLOCKS blocks"
[ "${LIVE_JOB_COUNT:-0}" -gt "$EXPECTED_SEED_HEIGHT" ] || fail "protocol did not generate synthetic jobs after seed"
[ "${LIVE_MODEL_COUNT:-0}" -gt 0 ] || fail "protocol did not register a LinearTrainingStep model"
[ "${LIVE_MODEL_STEP_TOTAL:-0}" -gt 0 ] || fail "protocol did not apply a LinearTrainingStep model transition"
[ "${LIVE_ATTESTATION_COUNT:-0}" -gt "$SEED_ATTESTATION_COUNT" ] || fail "live synthetic jobs did not add validator attestations"
[ "${LIVE_RECEIPT_COUNT:-0}" -gt "$EXPECTED_SETTLED_RECEIPTS" ] || fail "synthetic jobs did not produce additional receipts"
[ "${LIVE_SETTLED_RECEIPT_COUNT:-0}" -gt "$EXPECTED_SETTLED_RECEIPTS" ] || fail "synthetic jobs did not settle additional receipts"
[ "${LIVE_ATTESTED_RECEIPT_COUNT:-0}" -gt "$EXPECTED_SETTLED_RECEIPTS" ] || fail "live receipt details did not include validator attestations"
[ "${LIVE_TENSOR_OP_RECEIPT_COUNT:-0}" -gt "$EXPECTED_LIVE_PRIMITIVE_RECEIPT_FLOOR" ] || fail "live receipt details did not include post-seed TensorOp receipts"
[ "${LIVE_LINEAR_TRAINING_RECEIPT_COUNT:-0}" -gt "$EXPECTED_LIVE_PRIMITIVE_RECEIPT_FLOOR" ] || fail "live receipt details did not include post-seed LinearTrainingStep receipts"
[ "${LIVE_PENDING_RECEIPT_REWARD_COUNT:-0}" -gt "$SEED_PENDING_RECEIPT_REWARDS" ] || fail "live synthetic jobs did not add pending receipt rewards"
[ "${LIVE_PENDING_PROPOSER_REWARD_COUNT:-0}" -gt 0 ] || fail "live useful block proposals did not add delayed proposer rewards"
[ "${LIVE_DELAYED_RECEIPT_REWARD_CLAIMS:-0}" -gt 0 ] || fail "live synthetic jobs did not expose future-maturity pending receipt reward claims"
[ "${LIVE_DELAYED_PROPOSER_REWARD_CLAIMS:-0}" -gt 0 ] || fail "live useful block proposals did not expose future-maturity pending proposer reward claims"
[ "${LIVE_EXTERNAL_RANDOMNESS_BEACON_RECORDS:-0}" -gt 0 ] || fail "live runtime did not persist an external randomness beacon record"
[ "${LIVE_VALIDATOR_VRF_REVEAL_COUNT:-0}" -gt 0 ] || fail "live runtime did not persist validator VRF reveal records"
[ "${LIVE_LATEST_EXTERNAL_RANDOMNESS_BEACON_ROUND:-0}" -ge "$EXPECTED_RANDOMNESS_BEACON_ROUND" ] || fail "live runtime did not expose the configured external randomness beacon round"
[ "$LIVE_RANDOMNESS_CURRENT_BLOCK_HASH_ALLOWED" = "false" ] || fail "randomness evidence allowed current-block-hash randomness"
[ "$LIVE_RANDOMNESS_RECEIPT_ANCHORS_CONSISTENT" = "true" ] || fail "randomness evidence did not report consistent receipt anchors"

LIVE_TENSOR=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/latest")
LIVE_TENSOR_ID=$(json_string tensor_id "$LIVE_TENSOR")
[ -n "$LIVE_TENSOR_ID" ] || fail "live tensor route did not report a tensor id"
LIVE_TENSOR_ROOT=$(json_string root "$LIVE_TENSOR")
[ -n "$LIVE_TENSOR_ROOT" ] || fail "live tensor route did not report a tensor root"
[ "$(json_number tensor_count "$LIVE_TENSOR")" -gt 0 ] || fail "live tensor route did not report retained tensors"
LIVE_TENSOR_DESCRIPTOR=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/descriptor")
LIVE_TENSOR_DESCRIPTOR_ROOT=$(json_string root "$LIVE_TENSOR_DESCRIPTOR") \
  || fail "live tensor descriptor was not fetchable"
[ "$LIVE_TENSOR_DESCRIPTOR_ROOT" = "$LIVE_TENSOR_ROOT" ] || fail "live tensor descriptor root did not match latest tensor root"
LIVE_TENSOR_ROW=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/row/0")
[ "$(json_array_length row "$LIVE_TENSOR_ROW")" -gt 0 ] || fail "live tensor row was not fetchable"
LIVE_TENSOR_CHUNK=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/chunk/0")
LIVE_TENSOR_CHUNK_BYTES=$(json_string bytes "$LIVE_TENSOR_CHUNK") \
  || fail "live tensor chunk was not fetchable"
[ -n "$LIVE_TENSOR_CHUNK_BYTES" ] || fail "live tensor chunk was empty"
[ "$(json_number chunk_index "$LIVE_TENSOR_CHUNK")" = "0" ] || fail "live tensor chunk index did not match request"
LIVE_TENSOR_OPENING=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/opening/0")
LIVE_TENSOR_OPENING_PROOF_LEN=$(json_number proof_len "$LIVE_TENSOR_OPENING") \
  || fail "live tensor opening was not fetchable"
[ -n "$LIVE_TENSOR_OPENING_PROOF_LEN" ] || fail "live tensor opening did not report a proof length"
[ "$(json_number chunk_index "$LIVE_TENSOR_OPENING")" = "0" ] || fail "live tensor opening index did not match request"

LIVE_TENSOR_OP_BLOCK_HEIGHT=-1
LIVE_TENSOR_OP_BLOCK_RECEIPTS=0
LIVE_LINEAR_TRAINING_BLOCK_HEIGHT=0
LIVE_LINEAR_TRAINING_BLOCK_RECEIPTS=0
USEFUL_POW_BLOCK_EVIDENCE=false
CANONICAL_BLOCKSPACE_EVIDENCE=false
BLOCK_CHECKS_ROOT_EVIDENCE=false
VALIDATOR_PROPOSER_EVIDENCE=false
FINALITY_REQUIRES_USEFUL_POW=false
BLOCK_FINALITY_VOTE_EVIDENCE=false
attempt=0
while [ "$attempt" -lt "$EXPECTED_CHECKER_RETRY_LIMIT" ]; do
  LIVE_TENSOR_OP_BLOCK_HEIGHT=-1
  LIVE_TENSOR_OP_BLOCK_RECEIPTS=0
  LIVE_LINEAR_TRAINING_BLOCK_HEIGHT=0
  LIVE_LINEAR_TRAINING_BLOCK_RECEIPTS=0
  USEFUL_POW_BLOCK_EVIDENCE=false
  CANONICAL_BLOCKSPACE_EVIDENCE=false
  BLOCK_CHECKS_ROOT_EVIDENCE=false
  VALIDATOR_PROPOSER_EVIDENCE=false
  FINALITY_REQUIRES_USEFUL_POW=false
  BLOCK_FINALITY_VOTE_EVIDENCE=false
  LIVE_CHAIN_HEAD=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/chain/head")
  LIVE_HEIGHT=$(json_number height "$LIVE_CHAIN_HEAD")
  LIVE_OVERVIEW=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/explorer/overview")
  LIVE_FINALIZED_BLOCK_COUNT=$(json_summary_number finalized_block_count "$LIVE_OVERVIEW")
  BLOCK_SCAN_END=$((LIVE_FINALIZED_BLOCK_COUNT - 1))
  [ "$BLOCK_SCAN_END" -le "$LIVE_HEIGHT" ] || BLOCK_SCAN_END="$LIVE_HEIGHT"
  BLOCK_SCAN_START=$((BLOCK_SCAN_END - EXPECTED_BLOCK_SCAN_DEPTH))
  [ "$BLOCK_SCAN_START" -gt 0 ] || BLOCK_SCAN_START=0
  BLOCK_SCAN_HEIGHT="$BLOCK_SCAN_START"
  while [ "$BLOCK_SCAN_HEIGHT" -le "$BLOCK_SCAN_END" ]; do
  if BLOCK_RAW=$(read_service_block "$EXPECTED_BOOTSTRAP_SERVICE" "$BLOCK_SCAN_HEIGHT"); then
    BLOCK_STATUS="$BLOCK_RAW"
    BLOCK_FINALIZED=$(status_value finalized "$BLOCK_STATUS")
    BLOCK_RECEIPT_IDS=$(status_value receipt_ids "$BLOCK_STATUS")
    BLOCK_TENSOR_OP_RECEIPTS=$(status_value tensor_op_receipt_count "$BLOCK_STATUS")
    BLOCK_LINEAR_TRAINING_RECEIPTS=$(status_value linear_training_receipt_count "$BLOCK_STATUS")
    BLOCK_VALIDATION=$(status_value block_validation "$BLOCK_STATUS")
    BLOCK_POW_VALID=$(status_value pow_valid "$BLOCK_STATUS")
    BLOCK_CANONICAL_BLOCKSPACE_VALID=$(status_value canonical_blockspace_valid "$BLOCK_STATUS")
    BLOCK_SETTLED_RECEIPT_SET_ROOT=$(status_value settled_receipt_set_root "$BLOCK_STATUS")
    BLOCK_CHECKS_ROOT_RECOMPUTED=$(status_value checks_root_recomputed "$BLOCK_STATUS")
    BLOCK_CHECKS_ROOT=$(status_value checks_root "$BLOCK_STATUS")
    BLOCK_PROPOSER_REGISTERED=$(status_value proposer_registered "$BLOCK_STATUS")
    BLOCK_TENSORWORK_PROPOSER_SELECTION=$(status_value tensorwork_proposer_selection "$BLOCK_STATUS")
    BLOCK_FINALITY_VALIDATED=$(status_value finality_validated_block "$BLOCK_STATUS")
    BLOCK_VOTE_COUNT=$(status_value block_vote_count "$BLOCK_STATUS")
    BLOCK_VOTE_VALIDATORS=$(status_value block_vote_validators "$BLOCK_STATUS")
    BLOCK_VOTE_STAKE=$(status_value block_vote_stake "$BLOCK_STATUS")
    BLOCK_FINALITY_THRESHOLD_STAKE=$(status_value finality_threshold_stake "$BLOCK_STATUS")
    BLOCK_SELECTED_RECEIPT_COUNT=$(status_value selected_receipt_count "$BLOCK_STATUS")
    BLOCK_CHECK_LEAF_COUNT=$(status_value check_leaf_count "$BLOCK_STATUS")
    BLOCK_NONCE=$(status_value nonce "$BLOCK_STATUS")
    BLOCK_DIFFICULTY_TARGET=$(status_value difficulty_target "$BLOCK_STATUS")
    BLOCK_POW_HASH=$(status_value pow_hash "$BLOCK_STATUS")
    if [ "$BLOCK_FINALIZED" = "true" ] \
      && [ "$BLOCK_VALIDATION" = "useful_verification_pow" ] \
      && [ "$BLOCK_POW_VALID" = "true" ] \
      && [ -n "$BLOCK_NONCE" ] \
      && [ -n "$BLOCK_DIFFICULTY_TARGET" ] \
      && [ -n "$BLOCK_POW_HASH" ]; then
      USEFUL_POW_BLOCK_EVIDENCE=true
    fi
    if [ "$BLOCK_FINALIZED" = "true" ] \
      && [ "$BLOCK_CANONICAL_BLOCKSPACE_VALID" = "true" ] \
      && [ -n "$BLOCK_SETTLED_RECEIPT_SET_ROOT" ] \
      && [ -n "$BLOCK_SELECTED_RECEIPT_COUNT" ]; then
      CANONICAL_BLOCKSPACE_EVIDENCE=true
    fi
    if [ "$BLOCK_FINALIZED" = "true" ] \
      && [ "$BLOCK_CHECKS_ROOT_RECOMPUTED" = "true" ] \
      && [ -n "$BLOCK_CHECKS_ROOT" ] \
      && [ -n "$BLOCK_CHECK_LEAF_COUNT" ]; then
      BLOCK_CHECKS_ROOT_EVIDENCE=true
    fi
    if [ "$BLOCK_FINALIZED" = "true" ] \
      && [ "$BLOCK_PROPOSER_REGISTERED" = "true" ] \
      && [ "$BLOCK_TENSORWORK_PROPOSER_SELECTION" = "false" ]; then
      VALIDATOR_PROPOSER_EVIDENCE=true
    fi
    if [ "$BLOCK_FINALIZED" = "true" ] && [ "$BLOCK_FINALITY_VALIDATED" = "true" ]; then
      FINALITY_REQUIRES_USEFUL_POW=true
    fi
    if [ "$BLOCK_FINALIZED" = "true" ] \
      && is_u64 "$BLOCK_VOTE_COUNT" \
      && [ "$BLOCK_VOTE_COUNT" -gt 0 ] \
      && [ -n "$BLOCK_VOTE_VALIDATORS" ] \
      && [ "$BLOCK_VOTE_VALIDATORS" != "none" ] \
      && is_u64 "$BLOCK_VOTE_STAKE" \
      && is_u64 "$BLOCK_FINALITY_THRESHOLD_STAKE" \
      && [ "$BLOCK_VOTE_STAKE" -ge "$BLOCK_FINALITY_THRESHOLD_STAKE" ]; then
      BLOCK_FINALITY_VOTE_EVIDENCE=true
    fi
    if [ "$BLOCK_FINALIZED" = "true" ] \
      && [ -n "$BLOCK_RECEIPT_IDS" ] \
      && [ "$BLOCK_RECEIPT_IDS" != "none" ] \
      && [ "${BLOCK_TENSOR_OP_RECEIPTS:-0}" -gt 0 ]; then
      LIVE_TENSOR_OP_BLOCK_HEIGHT="$BLOCK_SCAN_HEIGHT"
      LIVE_TENSOR_OP_BLOCK_RECEIPTS="$BLOCK_TENSOR_OP_RECEIPTS"
    fi
    if [ "$BLOCK_FINALIZED" = "true" ] \
      && [ -n "$BLOCK_RECEIPT_IDS" ] \
      && [ "$BLOCK_RECEIPT_IDS" != "none" ] \
      && [ "${BLOCK_LINEAR_TRAINING_RECEIPTS:-0}" -gt 0 ]; then
      LIVE_LINEAR_TRAINING_BLOCK_HEIGHT="$BLOCK_SCAN_HEIGHT"
      LIVE_LINEAR_TRAINING_BLOCK_RECEIPTS="$BLOCK_LINEAR_TRAINING_RECEIPTS"
    fi
    if [ "$LIVE_TENSOR_OP_BLOCK_HEIGHT" -ge 0 ] \
      && [ "$LIVE_LINEAR_TRAINING_BLOCK_HEIGHT" -gt 0 ] \
      && [ "$USEFUL_POW_BLOCK_EVIDENCE" = "true" ] \
      && [ "$CANONICAL_BLOCKSPACE_EVIDENCE" = "true" ] \
      && [ "$BLOCK_CHECKS_ROOT_EVIDENCE" = "true" ] \
      && [ "$VALIDATOR_PROPOSER_EVIDENCE" = "true" ] \
      && [ "$FINALITY_REQUIRES_USEFUL_POW" = "true" ] \
      && [ "$BLOCK_FINALITY_VOTE_EVIDENCE" = "true" ]; then
      break
    fi
  fi
  if [ $((BLOCK_SCAN_HEIGHT % 20)) -eq 0 ]; then
    debug "block scan height=$BLOCK_SCAN_HEIGHT tensor_op=$LIVE_TENSOR_OP_BLOCK_HEIGHT linear=$LIVE_LINEAR_TRAINING_BLOCK_HEIGHT useful=$USEFUL_POW_BLOCK_EVIDENCE canonical=$CANONICAL_BLOCKSPACE_EVIDENCE checks_root=$BLOCK_CHECKS_ROOT_EVIDENCE proposer=$VALIDATOR_PROPOSER_EVIDENCE finality_pow=$FINALITY_REQUIRES_USEFUL_POW votes=$BLOCK_FINALITY_VOTE_EVIDENCE"
  fi
  BLOCK_SCAN_HEIGHT=$((BLOCK_SCAN_HEIGHT + 1))
  done
  if [ "$LIVE_TENSOR_OP_BLOCK_HEIGHT" -ge 0 ] \
    && [ "$LIVE_LINEAR_TRAINING_BLOCK_HEIGHT" -gt 0 ] \
    && [ "$USEFUL_POW_BLOCK_EVIDENCE" = "true" ] \
    && [ "$CANONICAL_BLOCKSPACE_EVIDENCE" = "true" ] \
    && [ "$BLOCK_CHECKS_ROOT_EVIDENCE" = "true" ] \
    && [ "$VALIDATOR_PROPOSER_EVIDENCE" = "true" ] \
    && [ "$FINALITY_REQUIRES_USEFUL_POW" = "true" ] \
    && [ "$BLOCK_FINALITY_VOTE_EVIDENCE" = "true" ]; then
    break
  fi
  debug "block scan attempt=$attempt start=$BLOCK_SCAN_START end=$BLOCK_SCAN_END tensor_op=$LIVE_TENSOR_OP_BLOCK_HEIGHT linear=$LIVE_LINEAR_TRAINING_BLOCK_HEIGHT useful=$USEFUL_POW_BLOCK_EVIDENCE canonical=$CANONICAL_BLOCKSPACE_EVIDENCE checks_root=$BLOCK_CHECKS_ROOT_EVIDENCE proposer=$VALIDATOR_PROPOSER_EVIDENCE finality_pow=$FINALITY_REQUIRES_USEFUL_POW votes=$BLOCK_FINALITY_VOTE_EVIDENCE"
  attempt=$((attempt + 1))
  sleep "$EXPECTED_CHECKER_RETRY_SLEEP_SECONDS"
done

[ "$LIVE_TENSOR_OP_BLOCK_HEIGHT" -ge 0 ] || fail "service block view did not expose finalized live TensorOp receipt evidence"
[ "$LIVE_LINEAR_TRAINING_BLOCK_HEIGHT" -gt 0 ] || fail "service block view did not expose finalized live LinearTrainingStep receipt evidence"
[ "$USEFUL_POW_BLOCK_EVIDENCE" = "true" ] || fail "service block view did not expose finalized useful-verification PoW evidence"
[ "$CANONICAL_BLOCKSPACE_EVIDENCE" = "true" ] || fail "service block view did not expose finalized canonical blockspace evidence"
[ "$BLOCK_CHECKS_ROOT_EVIDENCE" = "true" ] || fail "service block view did not expose finalized block checks-root evidence"
[ "$VALIDATOR_PROPOSER_EVIDENCE" = "true" ] || fail "service block view did not expose validator proposer evidence"
[ "$FINALITY_REQUIRES_USEFUL_POW" = "true" ] || fail "service block view did not expose useful-PoW finality validation evidence"
[ "$BLOCK_FINALITY_VOTE_EVIDENCE" = "true" ] || fail "service block view did not expose stake-weighted block vote finality evidence"

ALL_OPERATOR_NETWORK_HEAD_HEIGHT=""
ALL_OPERATOR_NETWORK_HEAD_HASH=""
ALL_OPERATOR_NETWORK_STATE_ROOT=""
attempt=0
while [ "$attempt" -lt "$EXPECTED_CHECKER_RETRY_LIMIT" ]; do
  TARGET_STATUS_RAW=$(read_service_status "$EXPECTED_NETWORK_OBSERVER_SERVICE") \
    || fail "could not read $EXPECTED_NETWORK_OBSERVER_SERVICE network-observed service status"
  TARGET_STATUS="$TARGET_STATUS_RAW"
  CANDIDATE_NETWORK_HEAD_HEIGHT=$(status_value role_p2p_latest_observed_block_payload_height "$TARGET_STATUS")
  CANDIDATE_NETWORK_HEAD_HASH=$(status_value role_p2p_latest_observed_block_payload_hash "$TARGET_STATUS")
  CANDIDATE_NETWORK_HASHES=$(status_value role_p2p_observed_block_payload_hashes "$TARGET_STATUS")
  CANDIDATE_FINALIZED_BLOCK_COUNT=$(status_value finalized_block_count "$TARGET_STATUS")
  if [ -n "$CANDIDATE_NETWORK_HEAD_HEIGHT" ] \
    && [ "$CANDIDATE_NETWORK_HEAD_HEIGHT" -ge "$EXPECTED_SEED_HEIGHT" ] \
    && [ -n "$CANDIDATE_NETWORK_HEAD_HASH" ] \
    && [ "$CANDIDATE_NETWORK_HEAD_HASH" != "unknown" ] \
    && [ "$CANDIDATE_NETWORK_HEAD_HASH" != "$ZERO_HASH" ] \
    && csv_contains_value "$CANDIDATE_NETWORK_HASHES" "$CANDIDATE_NETWORK_HEAD_HASH" \
    && is_u64 "$CANDIDATE_FINALIZED_BLOCK_COUNT" \
    && [ "$CANDIDATE_FINALIZED_BLOCK_COUNT" -gt "$EXPECTED_SEED_BLOCKS" ]; then
    NETWORK_TARGET_HEIGHT=$((CANDIDATE_FINALIZED_BLOCK_COUNT - 1))
    if NETWORK_BLOCK_RAW=$(read_service_block "$EXPECTED_NETWORK_OBSERVER_SERVICE" "$NETWORK_TARGET_HEIGHT"); then
      NETWORK_BLOCK_STATUS="$NETWORK_BLOCK_RAW"
      NETWORK_BLOCK_HEIGHT=$(status_value height "$NETWORK_BLOCK_STATUS")
      NETWORK_BLOCK_HASH=$(status_value block_hash "$NETWORK_BLOCK_STATUS")
      NETWORK_BLOCK_STATE_ROOT=$(status_value state_root "$NETWORK_BLOCK_STATUS")
      NETWORK_BLOCK_FINALIZED=$(status_value finalized "$NETWORK_BLOCK_STATUS")
      NETWORK_BLOCK_VOTE_COUNT=$(status_value block_vote_count "$NETWORK_BLOCK_STATUS")
      if [ -n "$NETWORK_BLOCK_HEIGHT" ] \
        && [ "$NETWORK_BLOCK_HEIGHT" = "$NETWORK_TARGET_HEIGHT" ] \
        && [ "$NETWORK_BLOCK_HEIGHT" -ge "$EXPECTED_SEED_HEIGHT" ] \
        && [ -n "$NETWORK_BLOCK_STATE_ROOT" ] \
        && [ "$NETWORK_BLOCK_STATE_ROOT" != "$ZERO_HASH" ] \
        && [ "$NETWORK_BLOCK_FINALIZED" = "true" ] \
        && is_u64 "$NETWORK_BLOCK_VOTE_COUNT" \
        && [ "$NETWORK_BLOCK_VOTE_COUNT" -gt 0 ]; then
        ALL_OPERATOR_NETWORK_HEAD_HEIGHT="$NETWORK_BLOCK_HEIGHT"
        ALL_OPERATOR_NETWORK_HEAD_HASH="$NETWORK_BLOCK_HASH"
        ALL_OPERATOR_NETWORK_STATE_ROOT="$NETWORK_BLOCK_STATE_ROOT"
        break
      fi
    fi
  fi
  debug "network target attempt=$attempt observed_height=${CANDIDATE_NETWORK_HEAD_HEIGHT:-none} observed_hash=${CANDIDATE_NETWORK_HEAD_HASH:-none} finalized_count=${CANDIDATE_FINALIZED_BLOCK_COUNT:-none} target_height=${NETWORK_TARGET_HEIGHT:-none} target_hash=${NETWORK_BLOCK_HASH:-none} finalized=${NETWORK_BLOCK_FINALIZED:-none} votes=${NETWORK_BLOCK_VOTE_COUNT:-none}"
  attempt=$((attempt + 1))
  sleep "$EXPECTED_CHECKER_RETRY_SLEEP_SECONDS"
done
[ -n "$ALL_OPERATOR_NETWORK_HEAD_HEIGHT" ] || fail "network-observed latest head height was not observed"
[ "$ALL_OPERATOR_NETWORK_HEAD_HEIGHT" -ge "$EXPECTED_SEED_HEIGHT" ] || fail "network-observed latest head did not reach seeded height $EXPECTED_SEED_HEIGHT"
[ -n "$ALL_OPERATOR_NETWORK_HEAD_HASH" ] || fail "network-observed latest head hash was not observed"
[ "$ALL_OPERATOR_NETWORK_HEAD_HASH" != "$ZERO_HASH" ] || fail "network-observed latest head hash was empty"
[ -n "$ALL_OPERATOR_NETWORK_STATE_ROOT" ] || fail "network-observed latest head state root was not observed"
[ "$ALL_OPERATOR_NETWORK_STATE_ROOT" != "$ZERO_HASH" ] || fail "network-observed latest head state root was empty"

ALL_OPERATOR_TARGET_HEAD_HEIGHT="$ALL_OPERATOR_NETWORK_HEAD_HEIGHT"
ALL_OPERATOR_TARGET_HEAD_HASH="$ALL_OPERATOR_NETWORK_HEAD_HASH"
ALL_OPERATOR_TARGET_STATE_ROOT="$ALL_OPERATOR_NETWORK_STATE_ROOT"

ALL_OPERATOR_MIN_HEIGHT=0
ALL_OPERATOR_FIRST_LIVE_BLOCK_HASH=""
ALL_OPERATOR_COMMON_HEAD_HEIGHT=0
ALL_OPERATOR_COMMON_HEAD_HASH=""
COMPETING_PROPOSER_SERVICES=""
CONVERGED_OPERATOR_COUNT=0
LIVE_ROLE_MINER_RECEIPT_OPERATOR_COUNT=0
LIVE_ROLE_MINER_TENSOR_OPERATOR_COUNT=0
LIVE_ROLE_MINER_RECEIPTS_SUBMITTED=0
LIVE_ROLE_MINER_TENSORS_INSERTED=0
LIVE_ROLE_VALIDATOR_ATTESTATION_OPERATOR_COUNT=0
LIVE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED=0
LIVE_ROLE_VALIDATOR_VRF_KEY_OPERATORS=0
LIVE_ROLE_VALIDATOR_VRF_KEY_REGISTRATIONS=0
LIVE_ROLE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT=0
LIVE_ROLE_DELAYED_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT=0
LIVE_ROLE_CHAIN_CADENCE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT=0
LIVE_ROLE_VALIDATOR_USEFUL_BLOCK_PROPOSER_OPERATOR_COUNT=0
LIVE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED=0
LIVE_ROLE_VALIDATOR_FALLBACK_BLOCKS_PROPOSED=0
LIVE_ROLE_VALIDATOR_PROPOSED_RECEIPTS=0
LIVE_LOCAL_SYNTHETIC_JOB_PRODUCER_COUNT=0
LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES=0
LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES_APPLIED=0
LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS=0
LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS_APPLIED=0
LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS=0
LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS_APPLIED=0
LIVE_ROLE_RANDOMNESS_BEACON_OPERATORS=0
LIVE_ROLE_RANDOMNESS_BEACONS_APPLIED=0
attempt=0
while [ "$attempt" -lt "$EXPECTED_OPERATOR_CONVERGENCE_RETRY_LIMIT" ]; do
  CONVERGED_OPERATOR_COUNT=0
  ALL_OPERATOR_MIN_HEIGHT=""
  ALL_OPERATOR_FIRST_LIVE_BLOCK_HASH=""
  COMPETING_PROPOSER_SERVICES=""
  LIVE_ROLE_MINER_RECEIPT_OPERATOR_COUNT=0
  LIVE_ROLE_MINER_TENSOR_OPERATOR_COUNT=0
  LIVE_ROLE_MINER_RECEIPTS_SUBMITTED=0
  LIVE_ROLE_MINER_TENSORS_INSERTED=0
  LIVE_ROLE_VALIDATOR_ATTESTATION_OPERATOR_COUNT=0
  LIVE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED=0
  LIVE_ROLE_VALIDATOR_VRF_KEY_OPERATORS=0
  LIVE_ROLE_VALIDATOR_VRF_KEY_REGISTRATIONS=0
  LIVE_ROLE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT=0
  LIVE_ROLE_DELAYED_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT=0
  LIVE_ROLE_CHAIN_CADENCE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT=0
  LIVE_ROLE_VALIDATOR_USEFUL_BLOCK_PROPOSER_OPERATOR_COUNT=0
  LIVE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED=0
  LIVE_ROLE_VALIDATOR_FALLBACK_BLOCKS_PROPOSED=0
  LIVE_ROLE_VALIDATOR_PROPOSED_RECEIPTS=0
  LIVE_LOCAL_SYNTHETIC_JOB_PRODUCER_COUNT=0
  LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES=0
  LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES_APPLIED=0
  LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS=0
  LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS_APPLIED=0
  LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS=0
  LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS_APPLIED=0
  LIVE_ROLE_RANDOMNESS_BEACON_OPERATORS=0
  LIVE_ROLE_RANDOMNESS_BEACONS_APPLIED=0
  STATUS_MISMATCH=false
  for service in $EXPECTED_SERVICES; do
    if STATUS_RAW=$(read_service_status "$service"); then
      STATUS="$STATUS_RAW"
    else
      STATUS_MISMATCH=true
      continue
    fi
    SERVICE_HEIGHT=$(status_value height "$STATUS")
    SERVICE_BLOCK_COUNT=$(status_value block_count "$STATUS")
    SERVICE_LATEST_BLOCK_HEIGHT=$(status_value latest_block_height "$STATUS")
    SERVICE_LATEST_BLOCK_HASH=$(status_value latest_block_hash "$STATUS")
    SERVICE_STATE_ROOT=$(status_value state_root "$STATUS")
    SERVICE_BLOCK_LOG_ROOT=$(status_value block_log_root "$STATUS")
    SERVICE_FINALIZED_BLOCK_COUNT=$(status_value finalized_block_count "$STATUS")
    SERVICE_FIRST_LIVE_BLOCK_HEIGHT=$(status_value first_live_block_height "$STATUS")
    SERVICE_FIRST_LIVE_BLOCK_HASH=$(status_value first_live_block_hash "$STATUS")
    SERVICE_ROLE=$(status_value role "$STATUS")
    SERVICE_REGISTERED_MINER_COUNT=$(status_value registered_miner_count "$STATUS")
    SERVICE_REGISTERED_VALIDATOR_COUNT=$(status_value registered_validator_count "$STATUS")
    SERVICE_JOB_COUNT=$(status_value job_count "$STATUS")
    SERVICE_RECEIPT_COUNT=$(status_value receipt_count "$STATUS")
    SERVICE_ATTESTATION_COUNT=$(status_value attestation_count "$STATUS")
    SERVICE_RUNTIME_COMMAND=$(status_value runtime_command "$STATUS")
    SERVICE_ROLE_RUNTIME_COMMAND=$(status_value role_runtime_command "$STATUS")
    SERVICE_ROLE_LOOP_READY=$(status_value role_loop_ready "$STATUS")
    SERVICE_ROLE_LOOP_ROLE=$(status_value role_loop_role "$STATUS")
    SERVICE_ROLE_CHAIN_PROFILE=$(status_value role_chain_profile "$STATUS")
    SERVICE_ROLE_CAN_PRODUCE_BLOCKS=$(status_value role_can_produce_blocks "$STATUS")
    SERVICE_ROLE_WALLET_ADDRESS=$(status_value role_wallet_address "$STATUS")
    SERVICE_ROLE_WALLET_REGISTRATION=$(status_value role_wallet_registration "$STATUS")
    SERVICE_ROLE_WALLET_REGISTERED=$(status_value role_wallet_registered "$STATUS")
    SERVICE_ROLE_MINER_WORK_READY=$(status_value role_miner_work_ready "$STATUS")
    SERVICE_ROLE_MINER_ASSIGNED_JOBS_SEEN=$(status_value role_miner_assigned_jobs_seen "$STATUS")
    SERVICE_ROLE_MINER_UNRECEIPTED_JOBS=$(status_value role_miner_unreceipted_jobs "$STATUS")
    SERVICE_ROLE_MINER_RECEIPTS_SUBMITTED=$(status_value role_miner_receipts_submitted "$STATUS")
    SERVICE_ROLE_MINER_TENSORS_INSERTED=$(status_value role_miner_tensors_inserted "$STATUS")
    SERVICE_ROLE_VALIDATOR_WORK_READY=$(status_value role_validator_work_ready "$STATUS")
    SERVICE_ROLE_VALIDATOR_ASSIGNED_RECEIPTS_SEEN=$(status_value role_validator_assigned_receipts_seen "$STATUS")
    SERVICE_ROLE_VALIDATOR_UNATTESTED_RECEIPTS=$(status_value role_validator_unattested_receipts "$STATUS")
    SERVICE_ROLE_VALIDATOR_ARTIFACT_READY_RECEIPTS=$(status_value role_validator_artifact_ready_receipts "$STATUS")
    SERVICE_ROLE_VALIDATOR_ARTIFACT_MISSING_RECEIPTS=$(status_value role_validator_artifact_missing_receipts "$STATUS")
    SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_ATTEMPTS=$(status_value role_validator_remote_tensor_fetch_attempts "$STATUS")
    SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_SUCCESSES=$(status_value role_validator_remote_tensor_fetch_successes "$STATUS")
    SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_FAILURES=$(status_value role_validator_remote_tensor_fetch_failures "$STATUS")
    SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_BYTES=$(status_value role_validator_remote_tensor_fetch_bytes "$STATUS")
    SERVICE_ROLE_VALIDATOR_REMOTE_TENSORS_INSERTED=$(status_value role_validator_remote_tensors_inserted "$STATUS")
    SERVICE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED=$(status_value role_validator_attestations_submitted "$STATUS")
    SERVICE_ROLE_VALIDATOR_VRF_KEY_REGISTERED=$(status_value role_validator_vrf_key_registered "$STATUS")
    SERVICE_ROLE_VALIDATOR_VRF_KEY_REGISTRATION_COUNT=$(status_value role_validator_vrf_key_registration_count "$STATUS")
    SERVICE_ROLE_VALIDATOR_VRF_PUBLIC_KEY=$(status_value role_validator_vrf_public_key "$STATUS")
    SERVICE_ROLE_VALIDATOR_PROPOSER_WORK_READY=$(status_value role_validator_proposer_work_ready "$STATUS")
    SERVICE_ROLE_VALIDATOR_PROPOSER_SETTLED_RECEIPTS_SEEN=$(status_value role_validator_proposer_settled_receipts_seen "$STATUS")
    SERVICE_ROLE_VALIDATOR_PROPOSER_ARTIFACT_READY_RECEIPTS_SEEN=$(status_value role_validator_proposer_artifact_ready_receipts_seen "$STATUS")
    SERVICE_ROLE_VALIDATOR_PROPOSER_ATTESTED_RECEIPTS_SEEN=$(status_value role_validator_proposer_attested_receipts_seen "$STATUS")
    SERVICE_ROLE_VALIDATOR_BLOCKS_PROPOSED=$(status_value role_validator_blocks_proposed "$STATUS")
    SERVICE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED=$(status_value role_validator_useful_blocks_proposed "$STATUS")
    SERVICE_ROLE_VALIDATOR_FALLBACK_BLOCKS_PROPOSED=$(status_value role_validator_fallback_blocks_proposed "$STATUS")
    SERVICE_ROLE_VALIDATOR_RECEIPTS_PROPOSED=$(status_value role_validator_receipts_proposed "$STATUS")
    SERVICE_ROLE_VALIDATOR_BLOCK_VOTES_SUBMITTED=$(status_value role_validator_block_votes_submitted "$STATUS")
    SERVICE_ROLE_RANDOMNESS_BEACON_MODE=$(status_value role_randomness_beacon_mode "$STATUS")
    SERVICE_ROLE_RANDOMNESS_BEACON_CONFIGURED=$(status_value role_randomness_beacon_configured "$STATUS")
    SERVICE_ROLE_RANDOMNESS_BEACON_SOURCE=$(status_value role_randomness_beacon_configured_source "$STATUS")
    SERVICE_ROLE_RANDOMNESS_BEACON_ROUND=$(status_value role_randomness_beacon_configured_round "$STATUS")
    SERVICE_ROLE_RANDOMNESS_BEACONS_OBSERVED=$(status_value role_randomness_beacons_observed "$STATUS")
    SERVICE_ROLE_RANDOMNESS_BEACONS_APPLIED=$(status_value role_randomness_beacons_applied "$STATUS")
    SERVICE_ROLE_RANDOMNESS_BEACONS_SKIPPED=$(status_value role_randomness_beacons_skipped "$STATUS")
    SERVICE_ROLE_RANDOMNESS_BEACON_FAILURES=$(status_value role_randomness_beacon_failures "$STATUS")
    SERVICE_ROLE_RANDOMNESS_LATEST_SOURCE_ID=$(status_value role_randomness_latest_source_id "$STATUS")
    SERVICE_ROLE_RANDOMNESS_LATEST_ROUND=$(status_value role_randomness_latest_round "$STATUS")
    SERVICE_ROLE_RANDOMNESS_LAST_ERROR=$(status_value role_randomness_last_error "$STATUS")
    SERVICE_ROLE_LOCAL_PRODUCER=$(status_value role_local_producer "$STATUS")
    SERVICE_ROLE_LOCAL_BLOCK_PROPOSER=$(status_value role_local_block_proposer "$STATUS")
    SERVICE_ROLE_LOCAL_BLOCK_PROPOSER_DELAY_BLOCKS=$(status_value role_local_block_proposer_delay_blocks "$STATUS")
    SERVICE_ROLE_LOCAL_BLOCK_PROPOSER_DELAY_SATISFIED=$(status_value role_local_block_proposer_delay_satisfied "$STATUS")
    SERVICE_ROLE_PROPOSER_COOLDOWN_BLOCKS=$(status_value role_proposer_cooldown_blocks "$STATUS")
    SERVICE_ROLE_PROPOSER_CADENCE_READY=$(status_value role_proposer_cadence_ready "$STATUS")
    SERVICE_ROLE_PROPOSER_CADENCE_REMAINING_BLOCKS=$(status_value role_proposer_cadence_remaining_blocks "$STATUS")
    SERVICE_ROLE_PRODUCED_BLOCKS=$(status_value role_produced_blocks "$STATUS")
    SERVICE_ROLE_NETWORK_APPLIED_BLOCKS=$(status_value role_network_applied_blocks "$STATUS")
    SERVICE_ROLE_NETWORK_EVENTS=$(status_value role_network_events_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_BLOCK_EVENTS=$(status_value role_network_block_events_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_BLOCK_HEADERS=$(status_value role_network_block_headers_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_BLOCK_PAYLOADS=$(status_value role_network_block_payloads_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_BLOCK_PAYLOADS_APPLIED=$(status_value role_network_block_payloads_applied "$STATUS")
    SERVICE_ROLE_NETWORK_BLOCK_VOTES=$(status_value role_network_block_votes_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_BLOCK_VOTES_APPLIED=$(status_value role_network_block_votes_applied "$STATUS")
    SERVICE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES=$(status_value role_network_block_check_challenges_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES_APPLIED=$(status_value role_network_block_check_challenges_applied "$STATUS")
    SERVICE_ROLE_NETWORK_JOB_EVENTS=$(status_value role_network_job_events_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_JOB_PAYLOADS=$(status_value role_network_job_payloads_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_JOB_PAYLOADS_APPLIED=$(status_value role_network_job_payloads_applied "$STATUS")
    SERVICE_ROLE_NETWORK_RECEIPT_EVENTS=$(status_value role_network_receipt_events_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS=$(status_value role_network_receipt_payloads_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS_APPLIED=$(status_value role_network_receipt_payloads_applied "$STATUS")
    SERVICE_ROLE_NETWORK_ATTESTATION_EVENTS=$(status_value role_network_attestation_events_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS=$(status_value role_network_attestation_payloads_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS_APPLIED=$(status_value role_network_attestation_payloads_applied "$STATUS")
    SERVICE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS=$(status_value role_network_external_randomness_beacons_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS_APPLIED=$(status_value role_network_external_randomness_beacons_applied "$STATUS")
    SERVICE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS=$(status_value role_network_validator_vrf_reveals_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS_APPLIED=$(status_value role_network_validator_vrf_reveals_applied "$STATUS")
    SERVICE_ROLE_NETWORK_PEER_EVENTS=$(status_value role_network_peer_events_ingested "$STATUS")
    SERVICE_ROLE_NETWORK_INVALID_EVENTS=$(status_value role_network_invalid_events "$STATUS")
    SERVICE_ROLE_LATEST_HEIGHT=$(status_value role_latest_height "$STATUS")
    SERVICE_ROLE_P2P_CONNECTED_PEERS=$(status_value role_p2p_connected_peers "$STATUS")
    SERVICE_ROLE_P2P_OBSERVED_BLOCKS=$(status_value role_p2p_observed_blocks "$STATUS")
    SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOADS=$(status_value role_p2p_observed_block_payloads "$STATUS")
    SERVICE_ROLE_P2P_OBSERVED_BLOCK_VOTES=$(status_value role_p2p_observed_block_votes "$STATUS")
    SERVICE_ROLE_P2P_OBSERVED_JOBS=$(status_value role_p2p_observed_jobs "$STATUS")
    SERVICE_ROLE_P2P_OBSERVED_RECEIPTS=$(status_value role_p2p_observed_receipts "$STATUS")
    SERVICE_ROLE_P2P_OBSERVED_ATTESTATIONS=$(status_value role_p2p_observed_attestations "$STATUS")
    SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HEIGHT=$(status_value role_p2p_latest_observed_block_height "$STATUS")
    SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HASH=$(status_value role_p2p_latest_observed_block_hash "$STATUS")
    SERVICE_ROLE_P2P_OBSERVED_BLOCK_HASHES=$(status_value role_p2p_observed_block_hashes "$STATUS")
    SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HEIGHT=$(status_value role_p2p_latest_observed_block_payload_height "$STATUS")
    SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HASH=$(status_value role_p2p_latest_observed_block_payload_hash "$STATUS")
    SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOAD_HASHES=$(status_value role_p2p_observed_block_payload_hashes "$STATUS")
    [ -n "$SERVICE_HEIGHT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_BLOCK_COUNT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_LATEST_BLOCK_HEIGHT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_LATEST_BLOCK_HASH" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_STATE_ROOT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_BLOCK_LOG_ROOT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_FINALIZED_BLOCK_COUNT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_FIRST_LIVE_BLOCK_HEIGHT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_FIRST_LIVE_BLOCK_HASH" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_REGISTERED_MINER_COUNT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_REGISTERED_VALIDATOR_COUNT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_JOB_COUNT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_RECEIPT_COUNT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ATTESTATION_COUNT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_RUNTIME_COMMAND" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_RUNTIME_COMMAND" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_LOOP_READY" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_LOOP_ROLE" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_CHAIN_PROFILE" = "local_cpu" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_CAN_PRODUCE_BLOCKS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_CAN_PRODUCE_BLOCKS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_WALLET_ADDRESS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_WALLET_ADDRESS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_WALLET_ADDRESS" != "none" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_WALLET_REGISTRATION" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_WALLET_REGISTRATION" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_WALLET_REGISTERED" = "true" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_MINER_WORK_READY" = "true" ] || [ "$SERVICE_ROLE_MINER_WORK_READY" = "false" ] || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_MINER_ASSIGNED_JOBS_SEEN" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_MINER_UNRECEIPTED_JOBS" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_MINER_RECEIPTS_SUBMITTED" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_MINER_TENSORS_INSERTED" || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_VALIDATOR_WORK_READY" = "true" ] || [ "$SERVICE_ROLE_VALIDATOR_WORK_READY" = "false" ] || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_ASSIGNED_RECEIPTS_SEEN" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_UNATTESTED_RECEIPTS" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_ARTIFACT_READY_RECEIPTS" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_ARTIFACT_MISSING_RECEIPTS" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_ATTEMPTS" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_SUCCESSES" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_FAILURES" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_BYTES" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_REMOTE_TENSORS_INSERTED" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED" || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_VALIDATOR_PROPOSER_WORK_READY" = "true" ] || [ "$SERVICE_ROLE_VALIDATOR_PROPOSER_WORK_READY" = "false" ] || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_PROPOSER_SETTLED_RECEIPTS_SEEN" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_PROPOSER_ARTIFACT_READY_RECEIPTS_SEEN" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_PROPOSER_ATTESTED_RECEIPTS_SEEN" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_BLOCKS_PROPOSED" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_FALLBACK_BLOCKS_PROPOSED" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_RECEIPTS_PROPOSED" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_VALIDATOR_BLOCK_VOTES_SUBMITTED" || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_LOCAL_PRODUCER" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_LOCAL_PRODUCER" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_LOCAL_PRODUCER" = "true" ] || [ "$SERVICE_ROLE_LOCAL_PRODUCER" = "false" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER" = "true" ] || [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER" = "false" ] || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER_DELAY_BLOCKS" || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER_DELAY_SATISFIED" = "true" ] || [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER_DELAY_SATISFIED" = "false" ] || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_PROPOSER_COOLDOWN_BLOCKS" || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_PROPOSER_COOLDOWN_BLOCKS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_PROPOSER_CADENCE_READY" = "true" ] || [ "$SERVICE_ROLE_PROPOSER_CADENCE_READY" = "false" ] || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_PROPOSER_CADENCE_REMAINING_BLOCKS" || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_PRODUCED_BLOCKS" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_APPLIED_BLOCKS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_APPLIED_BLOCKS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_EVENTS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_EVENTS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_BLOCK_EVENTS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_BLOCK_EVENTS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_BLOCK_HEADERS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_BLOCK_HEADERS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_BLOCK_PAYLOADS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_BLOCK_PAYLOADS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_BLOCK_PAYLOADS_APPLIED" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_BLOCK_PAYLOADS_APPLIED" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_BLOCK_VOTES" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_BLOCK_VOTES" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_BLOCK_VOTES_APPLIED" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_BLOCK_VOTES_APPLIED" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES_APPLIED" || { STATUS_MISMATCH=true; continue; }
    LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES=$((LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES + SERVICE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES))
    LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES_APPLIED=$((LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES_APPLIED + SERVICE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES_APPLIED))
    [ -n "$SERVICE_ROLE_NETWORK_JOB_EVENTS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_JOB_EVENTS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_JOB_PAYLOADS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_JOB_PAYLOADS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_JOB_PAYLOADS_APPLIED" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_JOB_PAYLOADS_APPLIED" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_RECEIPT_EVENTS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_RECEIPT_EVENTS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS_APPLIED" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS_APPLIED" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_ATTESTATION_EVENTS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_ATTESTATION_EVENTS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS_APPLIED" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS_APPLIED" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS_APPLIED" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS" || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS_APPLIED" || { STATUS_MISMATCH=true; continue; }
    LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS=$((LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS + SERVICE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS))
    LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS_APPLIED=$((LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS_APPLIED + SERVICE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS_APPLIED))
    LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS=$((LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS + SERVICE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS))
    LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS_APPLIED=$((LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS_APPLIED + SERVICE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS_APPLIED))
    [ -n "$SERVICE_ROLE_NETWORK_PEER_EVENTS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_PEER_EVENTS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_NETWORK_INVALID_EVENTS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_NETWORK_INVALID_EVENTS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    is_u64 "$SERVICE_ROLE_NETWORK_INVALID_EVENTS" || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_LATEST_HEIGHT" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_CONNECTED_PEERS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_CONNECTED_PEERS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_OBSERVED_BLOCKS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCKS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOADS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOADS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_VOTES" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_VOTES" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_OBSERVED_JOBS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_OBSERVED_JOBS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_OBSERVED_RECEIPTS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_OBSERVED_RECEIPTS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_OBSERVED_ATTESTATIONS" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_OBSERVED_ATTESTATIONS" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HEIGHT" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HEIGHT" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HASH" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HASH" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_HASHES" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_HASHES" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HEIGHT" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HEIGHT" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HASH" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HASH" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ -n "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOAD_HASHES" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOAD_HASHES" != "unknown" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_RANDOMNESS_BEACON_MODE" = "$EXPECTED_RANDOMNESS_BEACON_MODE" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_RANDOMNESS_BEACON_CONFIGURED" = "true" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_RANDOMNESS_BEACON_SOURCE" = "$EXPECTED_RANDOMNESS_BEACON_SOURCE_LABEL" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_RANDOMNESS_BEACON_ROUND" -eq "$EXPECTED_RANDOMNESS_BEACON_ROUND" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_RANDOMNESS_BEACONS_OBSERVED" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_RANDOMNESS_BEACONS_APPLIED" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_RANDOMNESS_BEACONS_SKIPPED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_RANDOMNESS_BEACON_FAILURES" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_RANDOMNESS_LATEST_SOURCE_ID" = "$EXPECTED_RANDOMNESS_BEACON_SOURCE_ID" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_RANDOMNESS_LATEST_ROUND" -eq "$EXPECTED_RANDOMNESS_BEACON_ROUND" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_RANDOMNESS_LAST_ERROR" = "none" ] || { STATUS_MISMATCH=true; continue; }
    LIVE_ROLE_RANDOMNESS_BEACON_OPERATORS=$((LIVE_ROLE_RANDOMNESS_BEACON_OPERATORS + 1))
    LIVE_ROLE_RANDOMNESS_BEACONS_APPLIED=$((LIVE_ROLE_RANDOMNESS_BEACONS_APPLIED + SERVICE_ROLE_RANDOMNESS_BEACONS_APPLIED))
    if [ "$SERVICE_ROLE_LOCAL_PRODUCER" != "true" ]; then
      [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_HASHES" != "none" ] || { STATUS_MISMATCH=true; continue; }
      [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOAD_HASHES" != "none" ] || { STATUS_MISMATCH=true; continue; }
    fi
    case "$service" in
      miner-*) [ "$SERVICE_ROLE" = "miner" ] || { STATUS_MISMATCH=true; continue; } ;;
      validator-*) [ "$SERVICE_ROLE" = "validator" ] || { STATUS_MISMATCH=true; continue; } ;;
    esac
    case "$service" in
      miner-*) [ "$SERVICE_RUNTIME_COMMAND" = "miner_run" ] || { STATUS_MISMATCH=true; continue; } ;;
      validator-*) [ "$SERVICE_RUNTIME_COMMAND" = "validator_run" ] || { STATUS_MISMATCH=true; continue; } ;;
    esac
    [ "$SERVICE_ROLE_RUNTIME_COMMAND" = "$SERVICE_RUNTIME_COMMAND" ] || { STATUS_MISMATCH=true; continue; }
    [ "$SERVICE_ROLE_LOOP_ROLE" = "$SERVICE_ROLE" ] || { STATUS_MISMATCH=true; continue; }
    case "$SERVICE_ROLE_LOOP_ROLE" in
      miner) ;;
      *)
        [ "$SERVICE_ROLE_MINER_WORK_READY" = "false" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_MINER_ASSIGNED_JOBS_SEEN" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_MINER_UNRECEIPTED_JOBS" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_MINER_RECEIPTS_SUBMITTED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_MINER_TENSORS_INSERTED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        ;;
    esac
    case "$SERVICE_ROLE_LOOP_ROLE" in
      miner)
        if [ "$SERVICE_ROLE_MINER_RECEIPTS_SUBMITTED" -gt 0 ]; then
          LIVE_ROLE_MINER_RECEIPT_OPERATOR_COUNT=$((LIVE_ROLE_MINER_RECEIPT_OPERATOR_COUNT + 1))
        fi
        if [ "$SERVICE_ROLE_MINER_TENSORS_INSERTED" -gt 0 ]; then
          LIVE_ROLE_MINER_TENSOR_OPERATOR_COUNT=$((LIVE_ROLE_MINER_TENSOR_OPERATOR_COUNT + 1))
        fi
        LIVE_ROLE_MINER_RECEIPTS_SUBMITTED=$((LIVE_ROLE_MINER_RECEIPTS_SUBMITTED + SERVICE_ROLE_MINER_RECEIPTS_SUBMITTED))
        LIVE_ROLE_MINER_TENSORS_INSERTED=$((LIVE_ROLE_MINER_TENSORS_INSERTED + SERVICE_ROLE_MINER_TENSORS_INSERTED))
        ;;
      validator)
        if [ "$SERVICE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED" -gt 0 ]; then
          LIVE_ROLE_VALIDATOR_ATTESTATION_OPERATOR_COUNT=$((LIVE_ROLE_VALIDATOR_ATTESTATION_OPERATOR_COUNT + 1))
        fi
        if [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER" = "true" ]; then
          LIVE_ROLE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT=$((LIVE_ROLE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT + 1))
          LIVE_ROLE_CHAIN_CADENCE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT=$((LIVE_ROLE_CHAIN_CADENCE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT + 1))
        fi
        if [ "$SERVICE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED" -gt 0 ]; then
          LIVE_ROLE_VALIDATOR_USEFUL_BLOCK_PROPOSER_OPERATOR_COUNT=$((LIVE_ROLE_VALIDATOR_USEFUL_BLOCK_PROPOSER_OPERATOR_COUNT + 1))
        fi
        LIVE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED=$((LIVE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED + SERVICE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED))
        LIVE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED=$((LIVE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED + SERVICE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED))
        LIVE_ROLE_VALIDATOR_FALLBACK_BLOCKS_PROPOSED=$((LIVE_ROLE_VALIDATOR_FALLBACK_BLOCKS_PROPOSED + SERVICE_ROLE_VALIDATOR_FALLBACK_BLOCKS_PROPOSED))
        LIVE_ROLE_VALIDATOR_PROPOSED_RECEIPTS=$((LIVE_ROLE_VALIDATOR_PROPOSED_RECEIPTS + SERVICE_ROLE_VALIDATOR_RECEIPTS_PROPOSED))
        ;;
    esac
    case "$SERVICE_ROLE_LOOP_ROLE" in
      validator) ;;
      *)
        [ "$SERVICE_ROLE_VALIDATOR_WORK_READY" = "false" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_ASSIGNED_RECEIPTS_SEEN" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_UNATTESTED_RECEIPTS" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_ARTIFACT_READY_RECEIPTS" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_ARTIFACT_MISSING_RECEIPTS" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_ATTEMPTS" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_SUCCESSES" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_FAILURES" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_BYTES" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_REMOTE_TENSORS_INSERTED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_PROPOSER_WORK_READY" = "false" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_PROPOSER_SETTLED_RECEIPTS_SEEN" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_PROPOSER_ARTIFACT_READY_RECEIPTS_SEEN" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_PROPOSER_ATTESTED_RECEIPTS_SEEN" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_BLOCKS_PROPOSED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_FALLBACK_BLOCKS_PROPOSED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_RECEIPTS_PROPOSED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_BLOCK_VOTES_SUBMITTED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        ;;
    esac
    case "$service" in
      miner-*) [ "$SERVICE_ROLE_WALLET_REGISTRATION" = "miner" ] || { STATUS_MISMATCH=true; continue; } ;;
      validator-*) [ "$SERVICE_ROLE_WALLET_REGISTRATION" = "validator" ] || { STATUS_MISMATCH=true; continue; } ;;
    esac
    case "$service" in
      validator-*)
        [ "$SERVICE_ROLE_VALIDATOR_VRF_KEY_REGISTERED" = "true" ] || { STATUS_MISMATCH=true; continue; }
        is_u64 "$SERVICE_ROLE_VALIDATOR_VRF_KEY_REGISTRATION_COUNT" || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_VRF_PUBLIC_KEY" != "none" ] || { STATUS_MISMATCH=true; continue; }
        LIVE_ROLE_VALIDATOR_VRF_KEY_OPERATORS=$((LIVE_ROLE_VALIDATOR_VRF_KEY_OPERATORS + 1))
        LIVE_ROLE_VALIDATOR_VRF_KEY_REGISTRATIONS=$((LIVE_ROLE_VALIDATOR_VRF_KEY_REGISTRATIONS + SERVICE_ROLE_VALIDATOR_VRF_KEY_REGISTRATION_COUNT))
        ;;
      miner-*)
        [ "$SERVICE_ROLE_VALIDATOR_VRF_KEY_REGISTERED" = "false" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_VRF_PUBLIC_KEY" = "none" ] || { STATUS_MISMATCH=true; continue; }
        ;;
    esac
    [ "$SERVICE_ROLE_LOOP_READY" = "true" ] || { STATUS_MISMATCH=true; continue; }
    case "$service" in
      validator-00)
        [ "$SERVICE_ROLE_CAN_PRODUCE_BLOCKS" = "true" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_PRODUCER" = "true" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER" = "true" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER_DELAY_BLOCKS" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER_DELAY_SATISFIED" = "true" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_PROPOSER_COOLDOWN_BLOCKS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
        ;;
      miner-*)
        [ "$SERVICE_ROLE_CAN_PRODUCE_BLOCKS" = "false" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_PRODUCER" = "false" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER" = "false" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER_DELAY_BLOCKS" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER_DELAY_SATISFIED" = "true" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_PRODUCED_BLOCKS" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_BLOCKS_PROPOSED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_FALLBACK_BLOCKS_PROPOSED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_RECEIPTS_PROPOSED" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_PROPOSER_ARTIFACT_READY_RECEIPTS_SEEN" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_VALIDATOR_PROPOSER_ATTESTED_RECEIPTS_SEEN" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        if restart_plateau_service_ready; then
          :
        elif [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER" = "false" ]; then
          [ "$SERVICE_ROLE_NETWORK_APPLIED_BLOCKS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_BLOCK_PAYLOADS_APPLIED" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_BLOCK_VOTES_APPLIED" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_EVENTS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_BLOCK_EVENTS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_BLOCK_HEADERS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_BLOCK_PAYLOADS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_BLOCK_VOTES" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_JOB_EVENTS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_JOB_PAYLOADS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_JOB_PAYLOADS_APPLIED" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_RECEIPT_EVENTS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS_APPLIED" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_ATTESTATION_EVENTS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS_APPLIED" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
        fi
        ;;
      validator-*)
        [ "$SERVICE_ROLE_CAN_PRODUCE_BLOCKS" = "true" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_PRODUCER" = "false" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER" = "true" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER_DELAY_BLOCKS" -eq 0 ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER_DELAY_SATISFIED" = "true" ] || { STATUS_MISMATCH=true; continue; }
        [ "$SERVICE_ROLE_PROPOSER_COOLDOWN_BLOCKS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
        if ! restart_plateau_service_ready; then
          [ "$SERVICE_ROLE_NETWORK_EVENTS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_BLOCK_EVENTS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_BLOCK_HEADERS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_BLOCK_PAYLOADS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_BLOCK_VOTES" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_JOB_EVENTS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_JOB_PAYLOADS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_JOB_PAYLOADS_APPLIED" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_RECEIPT_EVENTS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS_APPLIED" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_ATTESTATION_EVENTS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
          [ "$SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS_APPLIED" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
        fi
        ;;
    esac
    SERVICE_DELAYED_BLOCK_PROPOSER=false
    if [ "$SERVICE_ROLE_LOCAL_BLOCK_PROPOSER" = "true" ] \
      && [ "$SERVICE_ROLE_PROPOSER_COOLDOWN_BLOCKS" -gt 0 ]; then
      SERVICE_DELAYED_BLOCK_PROPOSER=true
    fi
    if [ "$SERVICE_REGISTERED_MINER_COUNT" -ne "$EXPECTED_MINER_COUNT" ] \
      || [ "$SERVICE_REGISTERED_VALIDATOR_COUNT" -ne "$EXPECTED_VALIDATOR_COUNT" ] \
      || [ "$SERVICE_JOB_COUNT" -le "$EXPECTED_SEED_HEIGHT" ] \
      || [ "$SERVICE_RECEIPT_COUNT" -le "$EXPECTED_SETTLED_RECEIPTS" ] \
      || [ "$SERVICE_ATTESTATION_COUNT" -le "$SEED_ATTESTATION_COUNT" ] \
      || [ "$SERVICE_ROLE_P2P_CONNECTED_PEERS" -le 0 ] \
      || { ! restart_plateau_service_ready \
        && { [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_VOTES" -le 0 ] \
          || [ "$SERVICE_ROLE_P2P_OBSERVED_JOBS" -le 0 ] \
          || [ "$SERVICE_ROLE_P2P_OBSERVED_RECEIPTS" -le 0 ] \
          || [ "$SERVICE_ROLE_P2P_OBSERVED_ATTESTATIONS" -le 0 ]; }; }; then
      STATUS_MISMATCH=true
      continue
    fi
    if [ "$SERVICE_HEIGHT" -le "$EXPECTED_SEED_HEIGHT" ] \
      || [ "$SERVICE_BLOCK_COUNT" -le "$EXPECTED_SEED_BLOCKS" ] \
      || [ "$SERVICE_LATEST_BLOCK_HEIGHT" -lt "$EXPECTED_SEED_HEIGHT" ] \
      || [ "$SERVICE_LATEST_BLOCK_HASH" = "$ZERO_HASH" ] \
      || [ "$SERVICE_STATE_ROOT" = "$ZERO_HASH" ] \
      || [ "$SERVICE_BLOCK_LOG_ROOT" = "$ZERO_HASH" ] \
      || [ "$SERVICE_ROLE_LATEST_HEIGHT" -le "$EXPECTED_SEED_HEIGHT" ]; then
      STATUS_MISMATCH=true
      continue
    fi
    if [ "$SERVICE_FINALIZED_BLOCK_COUNT" -le "$EXPECTED_SEED_BLOCKS" ]; then
      STATUS_MISMATCH=true
      continue
    fi
    if [ "$SERVICE_ROLE_LOCAL_PRODUCER" = "true" ]; then
      LIVE_LOCAL_SYNTHETIC_JOB_PRODUCER_COUNT=$((LIVE_LOCAL_SYNTHETIC_JOB_PRODUCER_COUNT + 1))
    elif restart_plateau_service_ready; then
      :
    else
      [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCKS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
      [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOADS" -gt 0 ] || { STATUS_MISMATCH=true; continue; }
      [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HEIGHT" -ge "$EXPECTED_SEED_HEIGHT" ] || { STATUS_MISMATCH=true; continue; }
      [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HASH" != "$ZERO_HASH" ] || { STATUS_MISMATCH=true; continue; }
      [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HEIGHT" -ge "$EXPECTED_SEED_HEIGHT" ] || { STATUS_MISMATCH=true; continue; }
      [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HASH" != "$ZERO_HASH" ] || { STATUS_MISMATCH=true; continue; }
      csv_contains_value "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOAD_HASHES" "$ALL_OPERATOR_NETWORK_HEAD_HASH" \
        || { STATUS_MISMATCH=true; continue; }
    fi
    SERVICE_COMMON_FINALIZED_HEIGHT=$((SERVICE_FINALIZED_BLOCK_COUNT - 1))
    if [ -z "$ALL_OPERATOR_MIN_HEIGHT" ] \
      || [ "$SERVICE_COMMON_FINALIZED_HEIGHT" -lt "$ALL_OPERATOR_MIN_HEIGHT" ]; then
      ALL_OPERATOR_MIN_HEIGHT="$SERVICE_COMMON_FINALIZED_HEIGHT"
    fi
    CONVERGED_OPERATOR_COUNT=$((CONVERGED_OPERATOR_COUNT + 1))
  done
  COMPETING_PROPOSER_COUNT=$(local_cpu_count_words $COMPETING_PROPOSER_SERVICES)
  EXPECTED_CONVERGED_OPERATOR_COUNT="$EXPECTED_SERVICE_COUNT"
  if [ "$CONVERGED_OPERATOR_COUNT" = "$EXPECTED_CONVERGED_OPERATOR_COUNT" ]; then
    COMMON_HEAD_MISMATCH=false
    TARGET_HEAD_MISMATCH=false
    ALL_OPERATOR_COMMON_HEAD_HEIGHT="$ALL_OPERATOR_MIN_HEIGHT"
    ALL_OPERATOR_COMMON_HEAD_HASH=""
    BLOCK_STATUS=""
    if BLOCK_RAW=$(read_service_block "$EXPECTED_NETWORK_OBSERVER_SERVICE" "$ALL_OPERATOR_COMMON_HEAD_HEIGHT"); then
      BLOCK_STATUS="$BLOCK_RAW"
    else
      COMMON_HEAD_MISMATCH=true
    fi
    SERVICE_COMMON_BLOCK_HASH=$(status_value block_hash "$BLOCK_STATUS")
    SERVICE_COMMON_BLOCK_FINALIZED=$(status_value finalized "$BLOCK_STATUS")
    [ -n "$SERVICE_COMMON_BLOCK_HASH" ] || COMMON_HEAD_MISMATCH=true
    [ "$SERVICE_COMMON_BLOCK_FINALIZED" = "true" ] || COMMON_HEAD_MISMATCH=true
    ALL_OPERATOR_COMMON_HEAD_HASH="$SERVICE_COMMON_BLOCK_HASH"
    BLOCK_STATUS=""
    if BLOCK_RAW=$(read_service_block "$EXPECTED_NETWORK_OBSERVER_SERVICE" "$ALL_OPERATOR_NETWORK_HEAD_HEIGHT"); then
      BLOCK_STATUS="$BLOCK_RAW"
    else
      TARGET_HEAD_MISMATCH=true
    fi
    SERVICE_TARGET_BLOCK_HASH=$(status_value block_hash "$BLOCK_STATUS")
    SERVICE_TARGET_STATE_ROOT=$(status_value state_root "$BLOCK_STATUS")
    SERVICE_TARGET_BLOCK_FINALIZED=$(status_value finalized "$BLOCK_STATUS")
    [ "$SERVICE_TARGET_BLOCK_HASH" = "$ALL_OPERATOR_NETWORK_HEAD_HASH" ] || TARGET_HEAD_MISMATCH=true
    [ "$SERVICE_TARGET_STATE_ROOT" = "$ALL_OPERATOR_NETWORK_STATE_ROOT" ] || TARGET_HEAD_MISMATCH=true
    [ "$SERVICE_TARGET_BLOCK_FINALIZED" = "true" ] || TARGET_HEAD_MISMATCH=true
    if [ "$COMMON_HEAD_MISMATCH" = "false" ] && [ "$TARGET_HEAD_MISMATCH" = "false" ]; then
      break
    fi
  fi
  debug "operator convergence attempt=$attempt converged=$CONVERGED_OPERATOR_COUNT expected=$EXPECTED_CONVERGED_OPERATOR_COUNT status_mismatch=$STATUS_MISMATCH competing_proposers=${COMPETING_PROPOSER_SERVICES:-none} common_head=${ALL_OPERATOR_COMMON_HEAD_HASH:-none} target_head=${ALL_OPERATOR_TARGET_HEAD_HASH:-none}"
  attempt=$((attempt + 1))
  sleep "$EXPECTED_CHECKER_RETRY_SLEEP_SECONDS"
done

[ "$CONVERGED_OPERATOR_COUNT" = "$EXPECTED_CONVERGED_OPERATOR_COUNT" ] || fail "not all operators satisfied local CPU role, gossip, and delayed proposer evidence"
[ -n "$ALL_OPERATOR_MIN_HEIGHT" ] || fail "operator convergence height was not observed"
[ "$ALL_OPERATOR_MIN_HEIGHT" -ge "$EXPECTED_SEED_HEIGHT" ] || fail "not all operators reached seeded height $EXPECTED_SEED_HEIGHT"
[ -n "$ALL_OPERATOR_COMMON_HEAD_HASH" ] || fail "operator common head hash convergence was not observed"
[ "$ALL_OPERATOR_COMMON_HEAD_HASH" != "$ZERO_HASH" ] || fail "operator common head hash convergence was empty"
[ -n "$ALL_OPERATOR_TARGET_HEAD_HASH" ] || fail "operator target latest head hash convergence was not observed"
[ "$ALL_OPERATOR_TARGET_HEAD_HASH" != "$ZERO_HASH" ] || fail "operator target latest head hash convergence was empty"
[ -n "$ALL_OPERATOR_TARGET_STATE_ROOT" ] || fail "operator target latest state-root convergence was not observed"
[ "$ALL_OPERATOR_TARGET_STATE_ROOT" != "$ZERO_HASH" ] || fail "operator target latest state-root convergence was empty"
[ -n "$ALL_OPERATOR_NETWORK_HEAD_HASH" ] || fail "operator network-observed latest head hash convergence was not observed"
[ "$ALL_OPERATOR_NETWORK_HEAD_HASH" != "$ZERO_HASH" ] || fail "operator network-observed latest head hash convergence was empty"
[ -n "$ALL_OPERATOR_NETWORK_STATE_ROOT" ] || fail "operator network-observed latest state-root convergence was not observed"
[ "$ALL_OPERATOR_NETWORK_STATE_ROOT" != "$ZERO_HASH" ] || fail "operator network-observed latest state-root convergence was empty"
if [ "$RESTART_CONTINUITY_MODE" != "true" ]; then
  [ "$LIVE_ROLE_MINER_RECEIPT_OPERATOR_COUNT" -gt 0 ] || fail "no miner role reported positive live receipt submissions"
  [ "$LIVE_ROLE_MINER_TENSOR_OPERATOR_COUNT" -gt 0 ] || fail "no miner role reported positive live tensor inserts"
  [ "$LIVE_ROLE_MINER_RECEIPTS_SUBMITTED" -gt 0 ] || fail "miner role receipt submission total did not advance"
  [ "$LIVE_ROLE_MINER_TENSORS_INSERTED" -gt 0 ] || fail "miner role tensor insert total did not advance"
  [ "$LIVE_ROLE_VALIDATOR_ATTESTATION_OPERATOR_COUNT" -gt 0 ] || fail "no validator role reported positive live attestation submissions"
  [ "$LIVE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED" -gt 0 ] || fail "validator role attestation submission total did not advance"
fi
[ "$LIVE_LOCAL_SYNTHETIC_JOB_PRODUCER_COUNT" -eq 1 ] || fail "expected exactly one local synthetic job producer"
[ "$LIVE_ROLE_VALIDATOR_VRF_KEY_OPERATORS" -eq "$EXPECTED_VALIDATOR_COUNT" ] || fail "not all validator roles registered production VRF keys"
if [ "$RESTART_CONTINUITY_MODE" != "true" ]; then
  [ "$LIVE_ROLE_VALIDATOR_VRF_KEY_REGISTRATIONS" -ge "$EXPECTED_VALIDATOR_COUNT" ] || fail "validator role VRF key registration total did not cover all validators"
fi
[ "$LIVE_ROLE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT" -ge 2 ] || fail "fewer than two validator role block proposers were enabled"
[ "$LIVE_ROLE_CHAIN_CADENCE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT" -gt 1 ] || fail "no chain-cadence validator role block proposer competition was enabled"
if [ "$RESTART_CONTINUITY_MODE" != "true" ]; then
  [ "$LIVE_ROLE_VALIDATOR_USEFUL_BLOCK_PROPOSER_OPERATOR_COUNT" -gt 0 ] || fail "no validator role proposed useful blocks"
  [ "$LIVE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED" -gt 0 ] || fail "validator role useful block proposal total did not advance"
  [ "$LIVE_ROLE_VALIDATOR_PROPOSED_RECEIPTS" -gt 0 ] || fail "validator role proposed receipt total did not advance"
  [ "$LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES_APPLIED" -gt 0 ] || fail "no role applied live diagnostic block-check challenges"
fi
[ "$LIVE_ROLE_RANDOMNESS_BEACON_OPERATORS" -eq "$EXPECTED_SERVICE_COUNT" ] || fail "not all operators applied the configured local randomness beacon"
[ "$LIVE_ROLE_RANDOMNESS_BEACONS_APPLIED" -ge "$EXPECTED_SERVICE_COUNT" ] || fail "operator randomness beacon apply total did not cover all operators"
if [ "$RESTART_CONTINUITY_MODE" != "true" ]; then
  [ "$LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS_APPLIED" -gt 0 ] || fail "no role applied external randomness beacon payloads from the network"
  [ "$LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS_APPLIED" -gt 0 ] || fail "no role applied validator VRF reveal payloads from the network"
fi

cat <<STATUS
local_cpu_testnet_ready=true
ready_miners=${EXPECTED_MINER_COUNT}
ready_validators=${EXPECTED_VALIDATOR_COUNT}
distinct_operator_ids=${EXPECTED_SERVICE_COUNT}
distinct_libp2p_peer_ids=${EXPECTED_SERVICE_COUNT}
distinct_node_multiaddrs=${EXPECTED_SERVICE_COUNT}
libp2p_ready_node_count=${EXPECTED_SERVICE_COUNT}
cpu_ready_miner_count=${EXPECTED_MINER_COUNT}
cuda_required_miner_count=${EXPECTED_CUDA_REQUIRED_MINER_COUNT}
settled_receipts=${EXPECTED_SETTLED_RECEIPTS}
matmul_settled=true
linear_training_settled=true
rewarded_miners=${SEED_REWARDED_MINERS}
pending_receipt_rewards=${SEED_PENDING_RECEIPT_REWARDS}
finality_rate_bps=${EXPECTED_FULL_RATE_BPS}
data_availability_bps=${EXPECTED_FULL_RATE_BPS}
standalone_explorer_ready=true
standalone_explorer_websocket_polling=true
live_block_production=true
live_synthetic_jobs=true
live_linear_training_jobs=true
live_model_step_total=${LIVE_MODEL_STEP_TOTAL}
live_attestations=true
live_receipt_attestations=true
live_tensor_op_receipts=true
live_linear_training_receipts=true
live_tensor_op_block_evidence=true
live_tensor_op_block_height=${LIVE_TENSOR_OP_BLOCK_HEIGHT}
live_tensor_op_block_receipts=${LIVE_TENSOR_OP_BLOCK_RECEIPTS}
live_linear_training_block_evidence=true
live_linear_training_block_height=${LIVE_LINEAR_TRAINING_BLOCK_HEIGHT}
live_linear_training_block_receipts=${LIVE_LINEAR_TRAINING_BLOCK_RECEIPTS}
live_tensor_fetch=true
live_rewards=true
live_pending_proposer_rewards=${LIVE_PENDING_PROPOSER_REWARD_COUNT}
live_delayed_receipt_reward_claims=${LIVE_DELAYED_RECEIPT_REWARD_CLAIMS}
live_delayed_proposer_reward_claims=${LIVE_DELAYED_PROPOSER_REWARD_CLAIMS}
live_pending_challenge_rewards=${LIVE_PENDING_CHALLENGE_REWARD_COUNT}
live_delayed_challenge_reward_claims=${LIVE_DELAYED_CHALLENGE_REWARD_CLAIMS}
live_external_randomness_beacon_records=${LIVE_EXTERNAL_RANDOMNESS_BEACON_RECORDS}
live_validator_vrf_reveals=${LIVE_VALIDATOR_VRF_REVEAL_COUNT}
live_role_validator_vrf_key_operators=${LIVE_ROLE_VALIDATOR_VRF_KEY_OPERATORS}
live_role_validator_vrf_key_registrations=${LIVE_ROLE_VALIDATOR_VRF_KEY_REGISTRATIONS}
live_latest_external_randomness_beacon_round=${LIVE_LATEST_EXTERNAL_RANDOMNESS_BEACON_ROUND}
live_randomness_current_block_hash_allowed=${LIVE_RANDOMNESS_CURRENT_BLOCK_HASH_ALLOWED}
live_randomness_receipt_anchors_consistent=${LIVE_RANDOMNESS_RECEIPT_ANCHORS_CONSISTENT}
all_operator_status_count=${EXPECTED_SERVICE_COUNT}
all_operator_min_height=${ALL_OPERATOR_MIN_HEIGHT}
all_operator_first_live_block_hash=${ALL_OPERATOR_FIRST_LIVE_BLOCK_HASH}
all_operator_live_block_convergence=true
all_operator_common_head_height=${ALL_OPERATOR_COMMON_HEAD_HEIGHT}
all_operator_common_head_hash=${ALL_OPERATOR_COMMON_HEAD_HASH}
all_operator_common_head_convergence=true
all_operator_target_head_height=${ALL_OPERATOR_TARGET_HEAD_HEIGHT}
all_operator_target_head_hash=${ALL_OPERATOR_TARGET_HEAD_HASH}
all_operator_target_state_root=${ALL_OPERATOR_TARGET_STATE_ROOT}
all_operator_target_head_convergence=true
all_operator_network_head_height=${ALL_OPERATOR_NETWORK_HEAD_HEIGHT}
all_operator_network_head_hash=${ALL_OPERATOR_NETWORK_HEAD_HASH}
all_operator_network_state_root=${ALL_OPERATOR_NETWORK_STATE_ROOT}
all_operator_network_head_convergence=true
all_operator_role_status=true
all_operator_role_runtime_commands=true
all_operator_role_wallets_registered=true
all_operator_miner_work_status=true
all_operator_miner_receipt_status=true
all_operator_validator_attestation_status=true
all_operator_validator_remote_tensor_fetch_status=true
all_operator_chain_profiles=true
all_operator_role_production_policy=true
all_operator_role_runtime_counters=true
live_role_miner_receipt_operators=${LIVE_ROLE_MINER_RECEIPT_OPERATOR_COUNT}
live_role_miner_tensor_operators=${LIVE_ROLE_MINER_TENSOR_OPERATOR_COUNT}
live_role_miner_receipts_submitted=${LIVE_ROLE_MINER_RECEIPTS_SUBMITTED}
live_role_miner_tensors_inserted=${LIVE_ROLE_MINER_TENSORS_INSERTED}
live_role_validator_attestation_operators=${LIVE_ROLE_VALIDATOR_ATTESTATION_OPERATOR_COUNT}
live_role_validator_attestations_submitted=${LIVE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED}
live_local_synthetic_job_producers=${LIVE_LOCAL_SYNTHETIC_JOB_PRODUCER_COUNT}
live_role_validator_block_proposer_operators=${LIVE_ROLE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT}
live_role_delayed_validator_block_proposer_operators=${LIVE_ROLE_DELAYED_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT}
live_role_chain_cadence_validator_block_proposer_operators=${LIVE_ROLE_CHAIN_CADENCE_VALIDATOR_BLOCK_PROPOSER_OPERATOR_COUNT}
live_role_validator_useful_block_proposer_operators=${LIVE_ROLE_VALIDATOR_USEFUL_BLOCK_PROPOSER_OPERATOR_COUNT}
live_competing_validator_block_proposers=${COMPETING_PROPOSER_SERVICES}
live_role_validator_useful_blocks_proposed=${LIVE_ROLE_VALIDATOR_USEFUL_BLOCKS_PROPOSED}
live_role_validator_fallback_blocks_proposed=${LIVE_ROLE_VALIDATOR_FALLBACK_BLOCKS_PROPOSED}
live_role_validator_proposed_receipts=${LIVE_ROLE_VALIDATOR_PROPOSED_RECEIPTS}
live_role_network_block_check_challenges=${LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES}
live_role_network_block_check_challenges_applied=${LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES_APPLIED}
live_role_network_external_randomness_beacons=${LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS}
live_role_network_external_randomness_beacons_applied=${LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS_APPLIED}
live_role_network_validator_vrf_reveals=${LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS}
live_role_network_validator_vrf_reveals_applied=${LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS_APPLIED}
live_role_randomness_beacon_operators=${LIVE_ROLE_RANDOMNESS_BEACON_OPERATORS}
live_role_randomness_beacons_applied=${LIVE_ROLE_RANDOMNESS_BEACONS_APPLIED}
live_role_owned_miner_receipts=true
live_role_owned_validator_attestations=true
single_local_synthetic_job_producer=true
multi_validator_proposer_competition=true
local_proposer_runtime=false
local_validator_producer=true
useful_pow_block_evidence=${USEFUL_POW_BLOCK_EVIDENCE}
canonical_blockspace_evidence=${CANONICAL_BLOCKSPACE_EVIDENCE}
block_checks_root_evidence=${BLOCK_CHECKS_ROOT_EVIDENCE}
validator_proposer_evidence=${VALIDATOR_PROPOSER_EVIDENCE}
tensorwork_proposer_selection_removed=true
finality_requires_useful_pow=${FINALITY_REQUIRES_USEFUL_POW}
block_vote_finality_evidence=${BLOCK_FINALITY_VOTE_EVIDENCE}
live_validator_proposer_networking=true
live_block_check_challenge_evidence=$([ "$LIVE_ROLE_NETWORK_BLOCK_CHECK_CHALLENGES_APPLIED" -gt 0 ] && printf '%s' true || printf '%s' false)
live_external_randomness_beacon_evidence=$([ "$LIVE_ROLE_RANDOMNESS_BEACON_OPERATORS" -eq "$EXPECTED_SERVICE_COUNT" ] && [ "${LIVE_EXTERNAL_RANDOMNESS_BEACON_RECORDS:-0}" -gt 0 ] && [ "$LIVE_ROLE_NETWORK_EXTERNAL_RANDOMNESS_BEACONS_APPLIED" -gt 0 ] && printf '%s' true || printf '%s' false)
live_validator_vrf_reveal_evidence=$([ "${LIVE_VALIDATOR_VRF_REVEAL_COUNT:-0}" -gt 0 ] && [ "$LIVE_ROLE_NETWORK_VALIDATOR_VRF_REVEALS_APPLIED" -gt 0 ] && printf '%s' true || printf '%s' false)
live_validator_block_vote_networking=true
all_non_producer_network_applied_blocks=true
all_non_producer_network_block_payload_ingestion=true
all_non_producer_network_block_payload_application=true
all_non_producer_network_block_vote_ingestion=true
all_non_producer_network_block_vote_application=true
all_non_producer_network_event_ingestion=true
all_non_producer_network_payload_announcements=true
all_non_producer_network_job_payload_application=true
all_non_producer_network_receipt_payload_application=true
all_non_producer_network_attestation_payload_application=true
all_operator_p2p_connected_peers=true
all_operator_p2p_block_gossip=true
all_operator_p2p_block_payload_gossip=true
all_operator_p2p_block_vote_gossip=true
all_operator_p2p_block_payload_head_observed=true
all_operator_p2p_job_gossip=true
all_operator_p2p_receipt_gossip=true
all_operator_p2p_attestation_gossip=true
all_operator_p2p_target_head_observed=true
all_operator_p2p_latest_head_observed=true
all_operator_chain_counters=true
all_operator_block_log_roots_observed=true
public_evidence_full_spec=false
independently_checkable=false
STATUS
