# syntax=docker/dockerfile:1

# rv64vm development toolchain.
#
#   - Rust (stable, pinned)     -> the VM itself (cargo build/test/run)
#   - riscv64 cross GCC         -> freestanding C kernel (kernel/)
#   - gdb-multiarch             -> debugging the kernel guest
#   - clang/gcc + cmake         -> C++20 host tooling (tools/)
#
# Base image is pinned by digest for reproducibility.

FROM rust:1.88-bookworm@sha256:af306cfa71d987911a781c37b59d7d67d934f49684058f96cf72079c3626bfe0

ARG UID=1000
ARG GID=1000

RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        gcc-riscv64-linux-gnu \
        binutils-riscv64-linux-gnu \
        gdb-multiarch \
        clang \
        cmake \
        ninja-build \
        file \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add rustfmt clippy

RUN (getent group "$GID" >/dev/null 2>&1 || groupadd --gid "$GID" dev) \
    && useradd --uid "$UID" --gid "$GID" --create-home --shell /bin/bash dev

ENV WORKDIR=/workspace
WORKDIR $WORKDIR

# The repo is bind-mounted at /workspace at runtime; the cargo registry is a
# named volume so dependency downloads survive container restarts.
USER dev
