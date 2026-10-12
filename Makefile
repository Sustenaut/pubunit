# The shared targets (venv, lock, fmt, lint, test, doc, build, functional)
# come from pubkit.mk, which pubkit manages; this file adds pubunit's own.
# SUPPLY-CHAIN.md is written for these crates by pubkit.mk's
# `make supply-chain`, and checked by `make supply-chain-check`.
SUPPLY_CHAIN_CRATES := pubunit

include pubkit.mk

# Binaries under test, by <NAME>_BIN_DIR.
export PUBUNIT_BIN_DIR ?= $(CURDIR)/target/debug

.DEFAULT_GOAL := help
.PHONY: check clean guard-floats ucum-table ucum-table-check vectors vectors-check doc-coverage doc-coverage-check fuzz-smoke deny vet publish-check msrv

check: sync-check fmt-check guard-floats ucum-table-check lint test doc doc-coverage-check functional vectors-check publish-check supply-chain-check vet deny ## Everything CI runs

clean: ## Remove build artifacts
	cargo clean
	rm -rf .pytest_cache tests/.pytest_cache

guard-floats: ## No f32 or f64 anywhere in src/
	./tools/check-floats.sh

ucum-table: ## Regenerate src/ucum_table.rs from the essence file ucum-units bundles
	python3 tools/gen-ucum-table.py

ucum-table-check: ## Fail if src/ucum_table.rs differs from the pinned ucum-units' essence file
	python3 tools/gen-ucum-table.py --check

vectors: ## Regenerate testdata/ from independent sources (NIST SP 811, the SI Brochure)
	python3 tools/vectors/generate.py

vectors-check: vectors ## Fail if the committed test vectors differ from what the sources give
	git diff --exit-code -- testdata

doc-coverage: ## Regenerate TESTING.md
	python3 tools/doc-coverage.py

doc-coverage-check: ## Fail if a public function has no example, a test covers no real item, or TESTING.md is stale
	python3 tools/doc-coverage.py --check

# What pubunit reads from outside: expressions and definitions (fuzz/).
FUZZ_TARGETS := parse reduce definitions
FUZZ_SECONDS ?= 60

fuzz-smoke: ## Replay fuzz regressions, then fuzz each target briefly (needs nightly and cargo-fuzz)
	cd fuzz && cargo +nightly fuzz build
	cd fuzz && for target in $(FUZZ_TARGETS); do \
	  mkdir -p corpus/$$target regressions/$$target; \
	  cargo +nightly fuzz run $$target corpus/$$target regressions/$$target -- \
	    -max_total_time=$(FUZZ_SECONDS) -timeout=10 || exit 1; \
	done

# The verify build gets a target directory of its own. Sharing target/
# leaves dep-info naming the packaged copy's sources, after which cargo
# reports pubunit Fresh however src/ changes, and tests run a stale binary.
publish-check: ## Package pubunit and verify it builds as published
	CARGO_TARGET_DIR=$(CURDIR)/target/publish-check $(CARGO) publish --dry-run --locked

vet: ## Every dependency is audited, trusted, or exempt with its evidence
	cargo vet --locked

deny: ## Licence and advisory audit
	$(CARGO) deny --locked check

# The oldest Rust pubunit promises: its rust-version. CI passes MSRV.
MSRV ?= $(shell sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml)

msrv: ## Build on the minimum Rust version (rustup toolchain install $(MSRV))
	$(CARGO) +$(MSRV) check --locked --all-targets
