.PHONY: check check-suite ci-structure-check fmt fmt-check lint test build run gallery gallery-shot qml-lint qml-lint-built token-gate contracts contracts-check bench-stream bench-history-load bench-frames hooks
.NOTPARALLEL:

CONTRACTS_REPO ?= https://github.com/GuilhermeFortuna/q_contracts.git
QMLLINT ?= $(shell find $(HOME)/.local/share/qt_minimal_download -name "qmllint" -type f 2>/dev/null | head -n 1 || command -v qmllint 2>/dev/null)

# Public entrypoint: routes through scripts/ci.sh for host ci.slice prioritization.
check:
	@./scripts/ci.sh

# Actual suite body (invoked by scripts/ci.sh after optional slice enter).
check-suite: ci-structure-check fmt-check lint test qml-lint-built token-gate contracts-check
	@echo "All terminal checks passed successfully."

ci-structure-check:
	@./scripts/check_ci_structure.sh

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

lint:
	cargo clippy --all-targets -- -D warnings

build:
	cargo build

qml-lint: build
	@$(MAKE) --no-print-directory qml-lint-built

qml-lint-built:
	@if [ -z "$(QMLLINT)" ]; then \
		echo "qmllint not found; please install qt6-declarative-dev-tools or use qt_minimal"; \
		exit 1; \
	fi; \
	$(QMLLINT) -W 0 \
		--import disable --missing-property disable --unresolved-type disable \
		--Quick.layout-positioning disable --unqualified disable \
		-i target/cxxqt/qml_modules/qml/qmldir $$(find qml -name '*.qml' -type f | sort)

token-gate:
	python3 tools/test_token_gate.py
	python3 tools/token_gate.py --check qml/

RUST_HOST := $(shell rustc -vV | awk '/^host:/ {print $$2}')
RUST_GCC_LD := $(shell rustc --print sysroot)/lib/rustlib/$(RUST_HOST)/bin/gcc-ld
CARGO_RUSTFLAGS := -C link-arg=-fuse-ld=lld -C link-arg=-B$(RUST_GCC_LD)
export RUSTFLAGS ?= $(CARGO_RUSTFLAGS)

test:
	cargo test -- --test-threads=1

bench-stream:
	cargo test --release --test bench_stream -- --nocapture --ignored

bench-history-load:
	cargo test --release --test bench_history_load -- --nocapture --ignored

run:
	cargo run

gallery:
	cargo run --features gallery -- --gallery

gallery-shot:
	QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software cargo run --features gallery -- --gallery-shot

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

hooks:
	@./scripts/install-hooks.sh
