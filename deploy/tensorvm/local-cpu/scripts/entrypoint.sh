#!/usr/bin/env sh
set -eu

DATA_DIR="${TENSORVM_DATA_DIR:-/var/lib/tensorvm}"
ROLE="${TENSORVM_ROLE:?TENSORVM_ROLE is required}"
OPERATOR_NAME="${TENSORVM_OPERATOR_NAME:?TENSORVM_OPERATOR_NAME is required}"
OPERATOR_ID="${TENSORVM_OPERATOR_ID:?TENSORVM_OPERATOR_ID is required}"
WALLET="${TENSORVM_WALLET:?TENSORVM_WALLET is required}"
NODE_MULTIADDR="${TENSORVM_NODE_MULTIADDR:?TENSORVM_NODE_MULTIADDR is required}"
P2P_LISTEN="${TENSORVM_P2P_LISTEN:-/ip4/0.0.0.0/tcp/4001}"
RPC_LISTEN="${TENSORVM_RPC_LISTEN:-0.0.0.0:8545}"
AUTH_TOKEN="${TENSORVM_AUTH_TOKEN:-local-cpu-testnet-token}"
MINER_STAKE="${TENSORVM_MINER_STAKE:-100}"
VALIDATOR_STAKE="${TENSORVM_VALIDATOR_STAKE:-10000}"
BOOTSTRAP_PEER_ID="${TENSORVM_BOOTSTRAP_PEER_ID:?TENSORVM_BOOTSTRAP_PEER_ID is required}"
BOOTSTRAP_ADDRESS="${TENSORVM_BOOTSTRAP_ADDRESS:-/dns4/miner-00/tcp/4001}"
IS_BOOTSTRAP="${TENSORVM_IS_BOOTSTRAP:-false}"
IDENTITY_SEED="${TENSORVM_LIBP2P_IDENTITY_SEED:-$OPERATOR_ID}"
SEED_LOCAL_TESTNET="${TENSORVM_SEED_LOCAL_TESTNET:-true}"
LOCAL_CPU_SYNTHETIC_JOB_PRODUCER="${TENSORVM_LOCAL_CPU_SYNTHETIC_JOB_PRODUCER:-false}"
LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER="${TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER:-false}"
LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS="${TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS:-0}"
LOCAL_CPU_PROPOSER_COOLDOWN_BLOCKS="${TENSORVM_LOCAL_CPU_PROPOSER_COOLDOWN_BLOCKS:-0}"
TENSORVM_CHAIN_PROFILE="${TENSORVM_CHAIN_PROFILE:-local_cpu}"
RUNTIME_COMMAND="${TENSORVM_ROLE_RUNTIME_COMMAND:-${ROLE}_run}"
READY_FILE="$DATA_DIR/local-cpu-ready"
INIT_OUT="/tmp/tensorvm-service-init.out"
export TENSORVM_CHAIN_PROFILE

mkdir -p "$DATA_DIR"
rm -f "$READY_FILE"

tvmd node init --data-dir "$DATA_DIR" > "$INIT_OUT"
cp "$INIT_OUT" "$DATA_DIR/service-init.out"

if [ "$IS_BOOTSTRAP" != "true" ]; then
  tvmd node peer add \
    --data-dir "$DATA_DIR" \
    --peer-id "$BOOTSTRAP_PEER_ID" \
    --address "$BOOTSTRAP_ADDRESS" > "$DATA_DIR/service-peer-add.out"
fi

if [ "${TENSORVM_LOCAL_CPU_STATIC_PEERS:-true}" = "true" ] && [ "$ROLE" = "validator" ]; then
  : > "$DATA_DIR/service-static-peers.out"
  while IFS='|' read -r peer_name peer_id peer_address; do
    [ -n "$peer_name" ] || continue
    [ "$peer_name" != "$OPERATOR_NAME" ] || continue
    tvmd node peer add \
      --data-dir "$DATA_DIR" \
      --peer-id "$peer_id" \
      --address "$peer_address" >> "$DATA_DIR/service-static-peers.out"
  done <<'PEERS'
validator-00|12D3KooWRCv6vs5HDE3ee5cesp61EisVgqEKZKzkKjaWbi18fCnJ|/dns4/validator-00/tcp/4001
validator-01|12D3KooWLhS5Dca2goQNapVtGib812k1RTA8XtJfCLpBAVsf5FGG|/dns4/validator-01/tcp/4001
validator-02|12D3KooWSYoDSK9eiELBNxJPdXWvt1FjHKoYETUCGTnZrQgLY4yU|/dns4/validator-02/tcp/4001
validator-03|12D3KooWARmdpZJdnFBZ6UwJXFx6rHfmxg33jYefT2jBBtwZkPSU|/dns4/validator-03/tcp/4001
validator-04|12D3KooWPWbHcg784TUAzyaWm3WD1tFGPSY2XnbXJGxHwG1y3JhU|/dns4/validator-04/tcp/4001
PEERS
fi

if [ "$SEED_LOCAL_TESTNET" = "true" ] && [ ! -f "$DATA_DIR/local-testnet-seed.out" ]; then
  tvmd localnet seed --data-dir "$DATA_DIR" > "$DATA_DIR/local-testnet-seed.out"
fi

case "$ROLE" in
  miner)
    if [ "$SEED_LOCAL_TESTNET" = "true" ]; then
      echo "role_registration=seeded_local_testnet" > "$DATA_DIR/role-register.out"
    else
      tvmd miner register --stake "$MINER_STAKE" > "$DATA_DIR/role-register.out"
    fi
    tvmd miner check \
      --wallet "$WALLET" \
      --device cpu \
      --node "$NODE_MULTIADDR" > "$DATA_DIR/role-start.out"
    ;;
  validator)
    if [ "$SEED_LOCAL_TESTNET" = "true" ]; then
      echo "role_registration=seeded_local_testnet" > "$DATA_DIR/role-register.out"
    else
      tvmd validator register --stake "$VALIDATOR_STAKE" > "$DATA_DIR/role-register.out"
    fi
    tvmd validator check \
      --wallet "$WALLET" \
      --node "$NODE_MULTIADDR" > "$DATA_DIR/role-start.out"
    ;;
  *)
    echo "unsupported TENSORVM_ROLE: $ROLE" >&2
    exit 2
    ;;
esac

tvmd node check \
  --p2p-listen "$P2P_LISTEN" \
  --data-dir "$DATA_DIR" \
  --identity-seed "$IDENTITY_SEED" > "$DATA_DIR/service-readiness.out"

{
  echo "operator_name=$OPERATOR_NAME"
  echo "operator_id=$OPERATOR_ID"
  echo "role=$ROLE"
  echo "runtime_command=$RUNTIME_COMMAND"
  echo "chain_profile=$TENSORVM_CHAIN_PROFILE"
  echo "local_cpu_synthetic_job_producer=$LOCAL_CPU_SYNTHETIC_JOB_PRODUCER"
  echo "local_cpu_validator_block_proposer=$LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER"
  echo "local_cpu_validator_block_proposer_delay_blocks=$LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS"
  echo "local_cpu_proposer_cooldown_blocks=$LOCAL_CPU_PROPOSER_COOLDOWN_BLOCKS"
  echo "node_multiaddr=$NODE_MULTIADDR"
  cat "$DATA_DIR/role-start.out"
  if [ -f "$DATA_DIR/local-testnet-seed.out" ]; then
    cat "$DATA_DIR/local-testnet-seed.out"
  fi
  cat "$DATA_DIR/service-readiness.out"
  echo "public_evidence_full_spec=false"
  echo "independently_checkable=false"
} > "$READY_FILE"

case "$ROLE" in
  miner)
    if [ "$RUNTIME_COMMAND" = "proposer_run" ]; then
      exec tvmd proposer run \
        --wallet "$WALLET" \
        --node "$NODE_MULTIADDR" \
        --listen "$RPC_LISTEN" \
        --p2p-listen "$P2P_LISTEN" \
        --data-dir "$DATA_DIR" \
        --identity-seed "$IDENTITY_SEED" \
        --auth-token "$AUTH_TOKEN" \
        --max-requests 0
    fi
    exec tvmd miner run \
      --wallet "$WALLET" \
      --device cpu \
      --node "$NODE_MULTIADDR" \
      --listen "$RPC_LISTEN" \
      --p2p-listen "$P2P_LISTEN" \
      --data-dir "$DATA_DIR" \
      --identity-seed "$IDENTITY_SEED" \
      --auth-token "$AUTH_TOKEN" \
      --max-requests 0
    ;;
  validator)
    exec tvmd validator run \
      --wallet "$WALLET" \
      --node "$NODE_MULTIADDR" \
      --listen "$RPC_LISTEN" \
      --p2p-listen "$P2P_LISTEN" \
      --data-dir "$DATA_DIR" \
      --identity-seed "$IDENTITY_SEED" \
      --auth-token "$AUTH_TOKEN" \
      --max-requests 0
    ;;
esac
