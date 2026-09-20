.PHONY: check check-suite fmt fmt-check lint test build run qml-lint contracts contracts-check bench-stream bench-history-load bench-frames
.NOTPARALLEL:

CONTRACTS_REPO ?= https://github.com/GuilhermeFortuna/q_contracts.git
QMLLINT ?= $(shell find $(HOME)/.local/share/qt_minimal_download -name "qmllint" -type f 2>/dev/null | head -n 1 || command -v qmllint 2>/dev/null)

# Public entrypoint: routes through scripts/ci.sh for host ci.slice prioritization.
check:
	@./scripts/ci.sh

# Actual suite body (invoked by scripts/ci.sh after optional slice enter).
check-suite: fmt-check lint build qml-lint test contracts-check
	cargo run -- --headless-report
	@echo "All terminal checks passed successfully."

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

lint:
	cargo clippy --all-targets -- -D warnings

build:
	cargo build

qml-lint: build
	@if [ -z "$(QMLLINT)" ]; then \
		echo "qmllint not found; please install qt6-declarative-dev-tools or use qt_minimal"; \
		exit 1; \
	fi; \
	$(QMLLINT) -W 0 -i target/cxxqt/qml_modules/qml/qmldir qml/Main.qml

RUST_HOST := $(shell rustc -vV | awk '/^host:/ {print $$2}')
RUST_GCC_LD := $(shell rustc --print sysroot)/lib/rustlib/$(RUST_HOST)/bin/gcc-ld
CARGO_TEST_RUSTFLAGS := -C link-arg=-fuse-ld=lld -C link-arg=-B$(RUST_GCC_LD)

test:
	RUSTFLAGS="$(CARGO_TEST_RUSTFLAGS)" cargo test

bench-stream:
	RUSTFLAGS="$(CARGO_TEST_RUSTFLAGS)" cargo test --release --test bench_stream -- --nocapture --ignored

bench-history-load:
	RUSTFLAGS="$(CARGO_TEST_RUSTFLAGS)" cargo test --release --test bench_history_load -- --nocapture --ignored

run:
	cargo run

bench-frames:
	cargo run -- --bench-frames --buckets $${BENCH_BUCKETS:-2000} --bars $${BENCH_BARS:-500000} --duration-ms $${BENCH_DURATION_MS:-300000} --execution-rows $${BENCH_EXECUTION_ROWS:-0} --markers $${BENCH_MARKERS:-0} --overlays $${BENCH_OVERLAYS:-0}

contracts:
	contracts_tmp="$$(mktemp -d)"; \
	trap 'rm -rf "$$contracts_tmp"' EXIT; \
	git clone --quiet "$(CONTRACTS_REPO)" "$$contracts_tmp/q_contracts"; \
	git -C "$$contracts_tmp/q_contracts" checkout --quiet "$$(cat CONTRACTS_REV)"; \
	rm -rf contracts; \
	mkdir -p contracts; \
	cp -R "$$contracts_tmp/q_contracts/generated/rust/." contracts/

contracts-check:
	contracts_tmp="$$(mktemp -d)"; \
	trap 'rm -rf "$$contracts_tmp"' EXIT; \
	git clone --quiet "$(CONTRACTS_REPO)" "$$contracts_tmp/q_contracts"; \
	git -C "$$contracts_tmp/q_contracts" checkout --quiet "$$(cat CONTRACTS_REV)"; \
	generated_tmp="$$contracts_tmp/generated"; \
	if python3 -c 'import yaml' 2>/dev/null; then \
		python3 "$$contracts_tmp/q_contracts/tools/generate.py" --language rust --out "$$generated_tmp"; \
	else \
		uv run --project "$$contracts_tmp/q_contracts" python "$$contracts_tmp/q_contracts/tools/generate.py" \
			--language rust --out "$$generated_tmp"; \
	fi; \
	diff -ru contracts "$$generated_tmp/rust"
