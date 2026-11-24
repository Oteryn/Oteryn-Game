# CI image for GitHub Actions jobs on Codex Cloud runners (label codex-cloud).
# Pins match .github/workflows/postgres-test-partitions.yml and rust-toolchain.toml;
# change them together.
FROM ubuntu:24.04@sha256:562111b88caa79c71391c0c4d3c2114a7d5c35915c20561fd893cb1e07929e6b

ARG RUSTUP_VERSION=1.29.3
ARG RUSTUP_SHA256=b1c8cb14e30889e7e2999e6b69e62d9e9b9ffc0f4df8180564d306d9680e6c55
ARG RUST_TOOLCHAIN=1.100.0
ARG NEXTEST_VERSION=0.9.146
ARG NEXTEST_SHA256=682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428
ARG MOLD_VERSION=2.42.1
ARG MOLD_SHA256=6ff270c9bf07d2bec5c98aa324eb7c4daf6a1a4d815c05ff1708049616047855

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/opt/rust/rustup \
    CARGO_HOME=/opt/rust/cargo \
    PATH=/opt/rust/cargo/bin:/opt/mold/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      build-essential ca-certificates curl git jq libasound2-dev libudev-dev \
      pkg-config python3 python3-yaml unzip xz-utils zstd \
 && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/rustup-init \
      "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/x86_64-unknown-linux-gnu/rustup-init" \
 && echo "${RUSTUP_SHA256}  /tmp/rustup-init" | sha256sum -c - \
 && chmod +x /tmp/rustup-init \
 && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain "${RUST_TOOLCHAIN}" \
      --component clippy --component rustfmt \
 && rm /tmp/rustup-init \
 && chmod -R a+rwX /opt/rust \
 && cargo --version && cargo clippy --version

RUN curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/nextest.tar.gz \
      "https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${NEXTEST_VERSION}/cargo-nextest-${NEXTEST_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
 && echo "${NEXTEST_SHA256}  /tmp/nextest.tar.gz" | sha256sum -c - \
 && tar -xzf /tmp/nextest.tar.gz -C /opt/rust/cargo/bin \
 && rm /tmp/nextest.tar.gz \
 && cargo nextest --version

RUN curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/mold.tar.gz \
      "https://github.com/rui314/mold/releases/download/v${MOLD_VERSION}/mold-${MOLD_VERSION}-x86_64-linux.tar.gz" \
 && echo "${MOLD_SHA256}  /tmp/mold.tar.gz" | sha256sum -c - \
 && mkdir -p /opt/mold \
 && tar -xzf /tmp/mold.tar.gz -C /opt/mold --strip-components=1 \
 && rm /tmp/mold.tar.gz \
 && mold --version

LABEL org.opencontainers.image.source="https://github.com/Oteryn/Oteryn-Game" \
      org.opencontainers.image.description="Oteryn-Game CI image for Codex Cloud runners"
