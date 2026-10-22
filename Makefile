# The shared targets come from pubkit.mk, which pubkit manages: the ones the
# Rust crate standard requires (`make required` lists what CI runs), and
# venv, lock, fmt, test, build and supply-chain. This file adds pubunit's own.
SUPPLY_CHAIN_CRATES := pubunit

include pubkit.mk

# Binaries under test, by <NAME>_BIN_DIR.
export PUBUNIT_BIN_DIR ?= $(CURDIR)/target/debug

.DEFAULT_GOAL := help
.PHONY: check clean guard-floats ucum-table ucum-table-check qudt-kinds qudt-kinds-check script-tests

check: sync-check fmt-check guard-floats ucum-table-check qudt-kinds-check lint test doc doc-coverage-check functional vectors-check publish-check supply-chain-check vet deny ## Everything CI runs

clean: ## Remove build artifacts
	cargo clean
	rm -rf .pytest_cache tests/.pytest_cache

guard-floats: ## No f32 or f64 anywhere in src/
	./tools/check-floats.sh

ucum-table: ## Regenerate src/ucum_table.generated.rs from the essence file ucum-units bundles
	python3 tools/gen-ucum-table.py

ucum-table-check: ## Fail if src/ucum_table.generated.rs differs from the pinned ucum-units' essence file
	python3 tools/gen-ucum-table.py --check

script-tests: ## Test the maintainer scripts; Cobertura XML in coverage/scripts.xml
	mkdir -p coverage
	coverage run --rcfile=tools/tests/coveragerc -m pytest -q tools/tests
	coverage xml --rcfile=tools/tests/coveragerc -o coverage/scripts.xml

qudt-kinds: build ## Regenerate data/qudt/ from the QUDT release data/qudt/source.json pins
	.venv/bin/python tools/gen-qudt-kinds.py

qudt-kinds-check: build ## Fail if data/qudt/ differs from what the pinned QUDT release gives
	.venv/bin/python tools/gen-qudt-kinds.py --check
