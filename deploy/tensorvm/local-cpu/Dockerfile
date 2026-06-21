FROM rust:1.94-bookworm AS builder

WORKDIR /workspace
COPY . .
RUN cargo build -p tensor_vm --release
RUN cargo build -p tensor_vm_explorer --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --home-dir /var/lib/tensorvm --shell /usr/sbin/nologin tensorvm \
    && install -d -o tensorvm -g tensorvm /var/lib/tensorvm

COPY --from=builder /workspace/target/release/tvmd /usr/local/bin/tvmd
COPY --from=builder /workspace/target/release/tensorvm-explorer /usr/local/bin/tensorvm-explorer
COPY deploy/tensorvm/local-cpu/scripts/entrypoint.sh /usr/local/bin/tensorvm-local-entrypoint

RUN chmod 0755 /usr/local/bin/tvmd /usr/local/bin/tensorvm-explorer /usr/local/bin/tensorvm-local-entrypoint

USER tensorvm
ENTRYPOINT ["/usr/local/bin/tensorvm-local-entrypoint"]
