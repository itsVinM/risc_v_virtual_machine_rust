# ── Stage 1: builder ──────────────────────────────────────────────────────
FROM rust:1.78-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig

WORKDIR /app
COPY . .

# Build headless release binary (no minifb display in container)
RUN cargo build --release --bin riscv-vm

# ── Stage 2: runtime ──────────────────────────────────────────────────────
FROM alpine:3.19

RUN apk add --no-cache libgcc

COPY --from=builder /app/target/release/riscv-vm /usr/local/bin/riscv-vm

ENTRYPOINT ["/usr/local/bin/riscv-vm"]
CMD ["--help"]
