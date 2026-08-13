# rv64vm — Dockerized dev entry points.
#
# Everything runs inside the pinned toolchain image so builds are identical
# on macOS (OrbStack) and CI. Usage:
#
#   make image        # build the toolchain image + init cargo cache
#   make build        # cargo build --release
#   make test         # cargo test
#   make vm ARGS="--sbi kernel/kernel"
#   make kernel       # cross-compile kernel/ with riscv64 gcc
#   make tools        # cmake + ctest for tools/
#   make fmt / lint   # rustfmt / clippy -D warnings
#   make bench        # cargo bench (Phase 3)
#   make shell        # interactive shell in the image

UID := $(shell id -u)
GID := $(shell id -g)

IMAGE := rv64vm:dev
CARGO_VOL := rv64vm-cargo-home
RUSTUP_VOL := rv64vm-rustup-home

# The cargo and rustup homes are mounted so crates and the toolchain synced by
# rust-toolchain.toml persist across container runs.
DOCKER_RUN := docker run --rm \
	--user $(UID):$(GID) \
	-v $(CURDIR):/workspace \
	-v $(CARGO_VOL):/usr/local/cargo \
	-v $(RUSTUP_VOL):/usr/local/rustup \
	$(IMAGE)

.PHONY: image cache build test vm kernel tools bench fmt lint shell

image:
	docker build --build-arg UID=$(UID) --build-arg GID=$(GID) -t $(IMAGE) .
	$(MAKE) cache

cache:
	docker volume create $(CARGO_VOL) >/dev/null 2>&1 || true
	docker volume create $(RUSTUP_VOL) >/dev/null 2>&1 || true
	docker run --rm -u root \
		-v $(CARGO_VOL):/usr/local/cargo \
		-v $(RUSTUP_VOL):/usr/local/rustup \
		$(IMAGE) sh -c 'chown -R $(UID):$(GID) /usr/local/cargo /usr/local/rustup'

build:
	$(DOCKER_RUN) cargo build --release

test:
	$(DOCKER_RUN) cargo test

vm:
	$(DOCKER_RUN) cargo run -- $(ARGS)

kernel:
	test -d kernel
	$(DOCKER_RUN) make -C kernel

tools:
	test -d tools
	$(DOCKER_RUN) bash -c 'cmake -S tools -B tools/build -G Ninja \
		&& cmake --build tools/build \
		&& ctest --test-dir tools/build --output-on-failure'

bench:
	$(DOCKER_RUN) cargo bench

fmt:
	$(DOCKER_RUN) cargo fmt --all -- --check

lint:
	$(DOCKER_RUN) cargo clippy --all-targets --all-features -- -D warnings

shell:
	$(DOCKER_RUN) bash
