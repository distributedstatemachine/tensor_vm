#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BUNDLE_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(CDPATH= cd -- "$BUNDLE_DIR/../../.." && pwd)
COMPOSE_FILE="$BUNDLE_DIR/docker-compose.yml"
CHECK_SCRIPT="$SCRIPT_DIR/check-local-testnet.sh"
TOPOLOGY_FILE="$SCRIPT_DIR/local-cpu-topology.sh"
AUTH_TOKEN="${TENSORVM_AUTH_TOKEN:-local-cpu-testnet-token}"
ZERO_HASH="0000000000000000000000000000000000000000000000000000000000000000"

fail() {
  echo "local CPU restart continuity check failed: $*" >&2
  exit 1
}

[ -r "$TOPOLOGY_FILE" ] || fail "local CPU topology file is not readable"
. "$TOPOLOGY_FILE"
EXPECTED_SERVICES="$LOCAL_CPU_EXPECTED_SERVICES"
DEFAULT_RESTART_SERVICES="$LOCAL_CPU_DEFAULT_RESTART_SERVICES"
EXPECTED_SEED_HEIGHT="$LOCAL_CPU_SEED_HEIGHT"
EXPECTED_RESTART_CHECKER_RETRY_LIMIT="$LOCAL_CPU_RESTART_CHECKER_RETRY_LIMIT"
EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS="$LOCAL_CPU_DOCKER_EXEC_TIMEOUT_SECONDS"
EXPECTED_HTTP_TIMEOUT_SECONDS="$LOCAL_CPU_HTTP_TIMEOUT_SECONDS"
EXPECTED_CHECKER_RETRY_SLEEP_SECONDS="$LOCAL_CPU_CHECKER_RETRY_SLEEP_SECONDS"
EXPECTED_RESTART_COMMAND_TIMEOUT_SECONDS="$LOCAL_CPU_RESTART_COMMAND_TIMEOUT_SECONDS"
EXPECTED_RESTART_CHECK_SCRIPT_TIMEOUT_SECONDS="$LOCAL_CPU_RESTART_CHECK_SCRIPT_TIMEOUT_SECONDS"
RESTART_SERVICES="${*:-$DEFAULT_RESTART_SERVICES}"

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

key_value_from_stdin() {
  key="$1"
  prefix="${key}="
  while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in
      "$prefix"*)
        printf '%s\n' "${line#"$prefix"}"
        return 0
        ;;
    esac
  done
  printf '\n'
}

status_value() {
  key="$1"
  document="$2"
  key_value_from_stdin "$key" <<EOF
$document
EOF
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

file_value() {
  key="$1"
  file="$2"
  key_value_from_stdin "$key" < "$file"
}

service_is_expected() {
  candidate="$1"
  for service in $EXPECTED_SERVICES; do
    [ "$candidate" = "$service" ] && return 0
  done
  return 1
}

read_service_status() {
  service="$1"
  attempt=0
  while [ "$attempt" -lt "$EXPECTED_RESTART_CHECKER_RETRY_LIMIT" ]; do
    if output=$(timeout "${EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node status --data-dir /var/lib/tensorvm 2>/dev/null < /dev/null); then
      printf '%s\n' "$output" | tr -d '\r'
      return 0
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
  while [ "$attempt" -lt "$EXPECTED_RESTART_CHECKER_RETRY_LIMIT" ]; do
    if output=$(timeout "${EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node block --data-dir /var/lib/tensorvm --height "$height" 2>/dev/null < /dev/null); then
      printf '%s\n' "$output" | tr -d '\r'
      return 0
    fi
    attempt=$((attempt + 1))
    sleep "$EXPECTED_CHECKER_RETRY_SLEEP_SECONDS"
  done
  return 1
}

wait_service_ready() {
  service="$1"
  attempt=0
  while [ "$attempt" -lt "$EXPECTED_RESTART_CHECKER_RETRY_LIMIT" ]; do
    if timeout "${EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" exec -T "$service" test -f /var/lib/tensorvm/local-cpu-ready 2>/dev/null < /dev/null; then
      return 0
    fi
    attempt=$((attempt + 1))
    sleep "$EXPECTED_CHECKER_RETRY_SLEEP_SECONDS"
  done
  return 1
}

service_rpc_get() {
  service="$1"
  path="$2"
  timeout "${EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" exec -T "$service" \
    curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:8545${path}" 2>/dev/null < /dev/null
}

capture_service_tensor() {
  phase="$1"
  service="$2"
  out_file="$3"
  attempt=0
  while [ "$attempt" -lt "$EXPECTED_RESTART_CHECKER_RETRY_LIMIT" ]; do
    if tensor=$(service_rpc_get "$service" /tensor/latest); then
      tensor_id=$(json_string tensor_id "$tensor") || tensor_id=""
      tensor_root=$(json_string root "$tensor") || tensor_root=""
      tensor_count=$(json_number tensor_count "$tensor") || tensor_count=0
      if [ -n "$tensor_id" ] && [ -n "$tensor_root" ] && [ "$tensor_count" -gt 0 ]; then
        {
          printf 'tensor_id=%s\n' "$tensor_id"
          printf 'tensor_root=%s\n' "$tensor_root"
          printf 'tensor_count=%s\n' "$tensor_count"
        } > "$out_file"
        return 0
      fi
    fi
    attempt=$((attempt + 1))
    sleep "$EXPECTED_CHECKER_RETRY_SLEEP_SECONDS"
  done
  fail "$phase tensor artifact was not available for $service"
}

assert_service_tensor_artifact() {
  service="$1"
  tensor_id="$2"
  tensor_root="$3"
  descriptor=$(service_rpc_get "$service" "/tensor/${tensor_id}/descriptor") \
    || fail "$service did not serve pre-restart tensor descriptor after restart"
  descriptor_root=$(json_string root "$descriptor") \
    || fail "$service pre-restart tensor descriptor did not expose root"
  [ "$descriptor_root" = "$tensor_root" ] \
    || fail "$service pre-restart tensor descriptor root changed after restart"
  row=$(service_rpc_get "$service" "/tensor/${tensor_id}/row/0") \
    || fail "$service did not serve pre-restart tensor row after restart"
  [ "$(json_array_length row "$row")" -gt 0 ] \
    || fail "$service pre-restart tensor row was empty after restart"
  chunk=$(service_rpc_get "$service" "/tensor/${tensor_id}/chunk/0") \
    || fail "$service did not serve pre-restart tensor chunk after restart"
  chunk_bytes=$(json_string bytes "$chunk") \
    || fail "$service pre-restart tensor chunk did not expose bytes"
  [ -n "$chunk_bytes" ] \
    || fail "$service pre-restart tensor chunk was empty after restart"
  [ "$(json_number chunk_index "$chunk")" = "0" ] \
    || fail "$service pre-restart tensor chunk index changed after restart"
  opening=$(service_rpc_get "$service" "/tensor/${tensor_id}/opening/0") \
    || fail "$service did not serve pre-restart tensor opening after restart"
  opening_proof_len=$(json_number proof_len "$opening") \
    || fail "$service pre-restart tensor opening did not expose proof length"
  [ -n "$opening_proof_len" ] \
    || fail "$service pre-restart tensor opening proof length was empty after restart"
  [ "$(json_number chunk_index "$opening")" = "0" ] \
    || fail "$service pre-restart tensor opening index changed after restart"
}

capture_snapshot() {
  phase="$1"
  dir="$2"
  min_height=""
  common_hash=""
  for service in $EXPECTED_SERVICES; do
    STATUS=$(read_service_status "$service") \
      || fail "could not read $phase service status for $service"
    peer_id=$(status_value p2p_peer_id "$STATUS")
    height=$(status_value height "$STATUS")
    block_count=$(status_value block_count "$STATUS")
    latest_block_height=$(status_value latest_block_height "$STATUS")
    latest_block_hash=$(status_value latest_block_hash "$STATUS")
    state_root=$(status_value state_root "$STATUS")
    block_log_root=$(status_value block_log_root "$STATUS")
    [ -n "$peer_id" ] || fail "$phase status for $service is missing p2p_peer_id"
    [ -n "$height" ] || fail "$phase status for $service is missing height"
    [ -n "$block_count" ] || fail "$phase status for $service is missing block_count"
    [ -n "$latest_block_height" ] || fail "$phase status for $service is missing latest_block_height"
    [ -n "$latest_block_hash" ] || fail "$phase status for $service is missing latest_block_hash"
    [ -n "$state_root" ] || fail "$phase status for $service is missing state_root"
    [ -n "$block_log_root" ] || fail "$phase status for $service is missing block_log_root"
    [ "$latest_block_height" -gt "$EXPECTED_SEED_HEIGHT" ] || fail "$phase status for $service latest block did not advance past seeded height $EXPECTED_SEED_HEIGHT"
    [ "$latest_block_hash" != "$ZERO_HASH" ] || fail "$phase status for $service has an empty latest block hash"
    [ "$state_root" != "$ZERO_HASH" ] || fail "$phase status for $service has an empty state root"
    [ "$block_log_root" != "$ZERO_HASH" ] || fail "$phase status for $service has an empty block-log root"
    if [ -z "$min_height" ] || [ "$latest_block_height" -lt "$min_height" ]; then
      min_height="$latest_block_height"
    fi
    {
      printf 'p2p_peer_id=%s\n' "$peer_id"
      printf 'height=%s\n' "$height"
      printf 'block_count=%s\n' "$block_count"
      printf 'latest_block_height=%s\n' "$latest_block_height"
      printf 'latest_block_hash=%s\n' "$latest_block_hash"
      printf 'state_root=%s\n' "$state_root"
      printf 'block_log_root=%s\n' "$block_log_root"
    } > "$dir/${service}.status"
  done

  [ -n "$min_height" ] || fail "$phase common head height was not observed"
  [ "$min_height" -gt "$EXPECTED_SEED_HEIGHT" ] || fail "$phase common head height did not advance past seeded height $EXPECTED_SEED_HEIGHT"
  common_state_root=""
  for service in $EXPECTED_SERVICES; do
    BLOCK_STATUS=$(read_service_block "$service" "$min_height") \
      || fail "could not read $phase common head block for $service"
    block_hash=$(status_value block_hash "$BLOCK_STATUS")
    state_root=$(status_value state_root "$BLOCK_STATUS")
    finalized=$(status_value finalized "$BLOCK_STATUS")
    [ -n "$block_hash" ] || fail "$phase common head block for $service is missing block_hash"
    [ -n "$state_root" ] || fail "$phase common head block for $service is missing state_root"
    [ "$block_hash" != "$ZERO_HASH" ] || fail "$phase common head block for $service has an empty hash"
    [ "$state_root" != "$ZERO_HASH" ] || fail "$phase common head block for $service has an empty state root"
    [ "$finalized" = "true" ] || fail "$phase common head block for $service is not finalized"
    if [ -z "$common_hash" ]; then
      common_hash="$block_hash"
    elif [ "$block_hash" != "$common_hash" ]; then
      fail "$phase common head block hash mismatch at height $min_height"
    fi
    if [ -z "$common_state_root" ]; then
      common_state_root="$state_root"
    elif [ "$state_root" != "$common_state_root" ]; then
      fail "$phase common head state-root mismatch at height $min_height"
    fi
  done
  {
    printf 'common_head_height=%s\n' "$min_height"
    printf 'common_head_hash=%s\n' "$common_hash"
    printf 'common_state_root=%s\n' "$common_state_root"
  } > "$dir/common.status"
}

cd "$REPO_ROOT"

require_command docker
require_command python3
require_command timeout

[ -x "$CHECK_SCRIPT" ] || fail "check-local-testnet.sh is not executable"
for service in $RESTART_SERVICES; do
  service_is_expected "$service" || fail "unexpected restart service: $service"
done

TMP_DIR="${TMPDIR:-/tmp}/tensorvm-local-cpu-restart.$$"
mkdir -p "$TMP_DIR/before" "$TMP_DIR/after"
trap 'rm -rf "$TMP_DIR"' EXIT INT TERM

capture_snapshot before "$TMP_DIR/before"
for service in $RESTART_SERVICES; do
  capture_service_tensor before "$service" "$TMP_DIR/before/${service}.tensor"
done

timeout "${EXPECTED_RESTART_COMMAND_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" restart $RESTART_SERVICES
for service in $RESTART_SERVICES; do
  wait_service_ready "$service" || fail "$service did not report local readiness after restart"
done

timeout "${EXPECTED_RESTART_CHECK_SCRIPT_TIMEOUT_SECONDS}s" "$CHECK_SCRIPT"

capture_snapshot after "$TMP_DIR/after"

BEFORE_COMMON_HEIGHT=$(file_value common_head_height "$TMP_DIR/before/common.status")
BEFORE_COMMON_HASH=$(file_value common_head_hash "$TMP_DIR/before/common.status")
BEFORE_COMMON_STATE_ROOT=$(file_value common_state_root "$TMP_DIR/before/common.status")
AFTER_COMMON_HEIGHT=$(file_value common_head_height "$TMP_DIR/after/common.status")
AFTER_COMMON_HASH=$(file_value common_head_hash "$TMP_DIR/after/common.status")
AFTER_COMMON_STATE_ROOT=$(file_value common_state_root "$TMP_DIR/after/common.status")

[ "$AFTER_COMMON_HEIGHT" -gt "$BEFORE_COMMON_HEIGHT" ] \
  || fail "blocks did not continue after restart"
[ "$AFTER_COMMON_HASH" != "$ZERO_HASH" ] \
  || fail "after-restart common head hash is empty"
[ "$AFTER_COMMON_STATE_ROOT" != "$ZERO_HASH" ] \
  || fail "after-restart common state root is empty"
[ "$AFTER_COMMON_STATE_ROOT" != "$BEFORE_COMMON_STATE_ROOT" ] \
  || fail "common state root did not advance after restart"

for service in $EXPECTED_SERVICES; do
  BLOCK_STATUS=$(read_service_block "$service" "$BEFORE_COMMON_HEIGHT") \
    || fail "could not reread pre-restart common head for $service"
  block_hash=$(status_value block_hash "$BLOCK_STATUS")
  state_root=$(status_value state_root "$BLOCK_STATUS")
  finalized=$(status_value finalized "$BLOCK_STATUS")
  [ "$block_hash" = "$BEFORE_COMMON_HASH" ] \
    || fail "$service does not preserve the pre-restart common head"
  [ "$state_root" = "$BEFORE_COMMON_STATE_ROOT" ] \
    || fail "$service does not preserve the pre-restart common state root"
  [ "$finalized" = "true" ] \
    || fail "$service pre-restart common head is not finalized after restart"
done

for service in $RESTART_SERVICES; do
  before_peer_id=$(file_value p2p_peer_id "$TMP_DIR/before/${service}.status")
  after_peer_id=$(file_value p2p_peer_id "$TMP_DIR/after/${service}.status")
  before_height=$(file_value height "$TMP_DIR/before/${service}.status")
  after_height=$(file_value height "$TMP_DIR/after/${service}.status")
  before_block_count=$(file_value block_count "$TMP_DIR/before/${service}.status")
  after_block_count=$(file_value block_count "$TMP_DIR/after/${service}.status")
  before_state_root=$(file_value state_root "$TMP_DIR/before/${service}.status")
  after_state_root=$(file_value state_root "$TMP_DIR/after/${service}.status")
  before_block_log_root=$(file_value block_log_root "$TMP_DIR/before/${service}.status")
  after_block_log_root=$(file_value block_log_root "$TMP_DIR/after/${service}.status")
  [ "$before_peer_id" = "$after_peer_id" ] \
    || fail "$service libp2p peer ID changed across restart"
  [ "$after_height" -gt "$before_height" ] \
    || fail "$service height did not advance across restart"
  [ "$after_block_count" -gt "$before_block_count" ] \
    || fail "$service block count did not advance across restart"
  [ "$after_state_root" != "$ZERO_HASH" ] \
    || fail "$service after-restart state root is empty"
  [ "$after_state_root" != "$before_state_root" ] \
    || fail "$service state root did not advance across restart"
  [ "$after_block_log_root" != "$ZERO_HASH" ] \
    || fail "$service after-restart block-log root is empty"
  [ "$after_block_log_root" != "$before_block_log_root" ] \
    || fail "$service block-log root did not advance across restart"
  before_tensor_id=$(file_value tensor_id "$TMP_DIR/before/${service}.tensor")
  before_tensor_root=$(file_value tensor_root "$TMP_DIR/before/${service}.tensor")
  assert_service_tensor_artifact "$service" "$before_tensor_id" "$before_tensor_root"
done

RESTART_SERVICE_LIST=$(printf '%s\n' "$RESTART_SERVICES" | tr ' ' ',')

cat <<STATUS
local_cpu_restart_continuity_ready=true
restart_services=${RESTART_SERVICE_LIST}
before_common_head_height=${BEFORE_COMMON_HEIGHT}
before_common_head_hash=${BEFORE_COMMON_HASH}
before_common_state_root=${BEFORE_COMMON_STATE_ROOT}
after_common_head_height=${AFTER_COMMON_HEIGHT}
after_common_head_hash=${AFTER_COMMON_HASH}
after_common_state_root=${AFTER_COMMON_STATE_ROOT}
restart_peer_ids_stable=true
restart_heights_non_decreasing=true
restart_heights_advance=true
restart_block_counts_non_decreasing=true
restart_block_counts_advance=true
restart_state_roots_observed=true
restart_state_roots_advance=true
restart_block_log_roots_observed=true
restart_block_log_roots_advance=true
restart_previous_common_head_preserved=true
restart_previous_common_state_root_preserved=true
restart_tensor_artifact_preserved=true
restart_blocks_continue=true
restart_common_head_convergence=true
STATUS
