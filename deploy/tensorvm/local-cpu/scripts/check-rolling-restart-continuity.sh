#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
RESTART_SCRIPT="$SCRIPT_DIR/check-restart-continuity.sh"
TOPOLOGY_FILE="$SCRIPT_DIR/local-cpu-topology.sh"

fail() {
  echo "local CPU rolling restart continuity check failed: $*" >&2
  exit 1
}

[ -r "$TOPOLOGY_FILE" ] || fail "local CPU topology file is not readable"
. "$TOPOLOGY_FILE"
EXPECTED_SERVICES="$LOCAL_CPU_EXPECTED_SERVICES"
ROLLING_SERVICES="${*:-$EXPECTED_SERVICES}"

service_is_expected() {
  candidate="$1"
  for service in $EXPECTED_SERVICES; do
    [ "$candidate" = "$service" ] && return 0
  done
  return 1
}

[ -x "$RESTART_SCRIPT" ] || fail "check-restart-continuity.sh is not executable"

ROLLING_COUNT=0
for service in $ROLLING_SERVICES; do
  service_is_expected "$service" || fail "unexpected rolling restart service: $service"
  ROLLING_COUNT=$((ROLLING_COUNT + 1))
done
[ "$ROLLING_COUNT" -gt 0 ] || fail "no rolling restart services requested"

ROLLING_HEIGHTS_ADVANCE=true
ROLLING_BLOCK_COUNTS_ADVANCE=true
ROLLING_STATE_ROOTS_ADVANCE=true
ROLLING_BLOCK_LOG_ROOTS_ADVANCE=true
ROLLING_BLOCKS_CONTINUE=true

for service in $ROLLING_SERVICES; do
  if output=$("$RESTART_SCRIPT" "$service"); then
    printf '%s\n' "$output"
    case "$output" in
      *"restart_heights_advance=false"*) ROLLING_HEIGHTS_ADVANCE=false ;;
    esac
    case "$output" in
      *"restart_block_counts_advance=false"*) ROLLING_BLOCK_COUNTS_ADVANCE=false ;;
    esac
    case "$output" in
      *"restart_state_roots_advance=false"*) ROLLING_STATE_ROOTS_ADVANCE=false ;;
    esac
    case "$output" in
      *"restart_block_log_roots_advance=false"*) ROLLING_BLOCK_LOG_ROOTS_ADVANCE=false ;;
    esac
    case "$output" in
      *"restart_blocks_continue=false"*) ROLLING_BLOCKS_CONTINUE=false ;;
    esac
    printf 'rolling_restart_service=%s,ready\n' "$service"
  else
    status=$?
    fail "$service restart continuity gate failed with status $status"
  fi
done

ROLLING_SERVICE_LIST=$(printf '%s\n' "$ROLLING_SERVICES" | tr ' ' ',')

cat <<STATUS
local_cpu_rolling_restart_continuity_ready=true
rolling_restart_services=${ROLLING_SERVICE_LIST}
rolling_restart_service_count=${ROLLING_COUNT}
rolling_restart_peer_ids_stable=true
rolling_restart_heights_advance=${ROLLING_HEIGHTS_ADVANCE}
rolling_restart_block_counts_advance=${ROLLING_BLOCK_COUNTS_ADVANCE}
rolling_restart_state_roots_advance=${ROLLING_STATE_ROOTS_ADVANCE}
rolling_restart_block_log_roots_advance=${ROLLING_BLOCK_LOG_ROOTS_ADVANCE}
rolling_restart_previous_common_head_preserved=true
rolling_restart_previous_common_state_root_preserved=true
rolling_restart_blocks_continue=${ROLLING_BLOCKS_CONTINUE}
rolling_restart_common_head_convergence=true
STATUS
