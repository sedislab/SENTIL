.PHONY: build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean test-embedded verify bench-smc bench-modest bench-uppaal bench-prism baselines

MODEST ?= modest
PRISM ?= prism
VERIFYTA ?= verifyta

build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean:
	$(MAKE) -C sentil-ffi $@

test-embedded:
	$(MAKE) -C sentil-embedded test

verify:
	python3 scripts/check_version.py
	cargo test --offline -p sentil
	$(MAKE) -C sentil-ffi test-ffi CARGO='cargo --offline'
	python3 scripts/check_claims.py

bench-modest:
	@tmp=$$(mktemp); trap 'rm -f "$$tmp"' EXIT; \
	for m in biodiesel circadian powertrain tandem_queue; do \
	  MODEST="$(MODEST)" bash benchmarks/runners/modest_runner.sh benchmarks/baselines/modest/$$m.modest >>"$$tmp" || exit 1; \
	done; \
	if [ -s "$$tmp" ]; then cp "$$tmp" benchmarks/results/modest_smc.jsonl; fi

bench-uppaal:
	@tmp=$$(mktemp); trap 'rm -f "$$tmp"' EXIT; \
	for m in benchmarks/baselines/uppaal/*.xml; do \
	  VERIFYTA="$(VERIFYTA)" bash benchmarks/runners/uppaal_runner.sh "$$m" >>"$$tmp" || exit 1; \
	done; \
	if [ -s "$$tmp" ]; then cp "$$tmp" benchmarks/results/uppaal_smc.jsonl; fi

bench-prism:
	@tmp=$$(mktemp); trap 'rm -f "$$tmp"' EXIT; \
	for m in benchmarks/baselines/prism/*.nm; do \
	  PRISM="$(PRISM)" bash benchmarks/runners/prism_runner.sh "$$m" >>"$$tmp" || exit 1; \
	done; \
	if [ -s "$$tmp" ]; then cp "$$tmp" benchmarks/results/prism_smc.jsonl; fi

bench-smc:
	@tmp=$$(mktemp); trap 'rm -f "$$tmp"' EXIT; \
	for m in circadian tandem_queue biodiesel powertrain; do \
	  cargo run --release -q -p sentil-benchmarks --bin sentil_ctmc_runner -- $$m >>"$$tmp" || exit 1; \
	done; \
	if [ -s "$$tmp" ]; then cp "$$tmp" benchmarks/results/sentil_smc.jsonl; fi

baselines:
	@ran=0; \
	if command -v "$(MODEST)" >/dev/null 2>&1; then \
	  rm -f benchmarks/results/modest_smc.jsonl; \
	  $(MAKE) bench-modest MODEST="$(MODEST)"; \
	  n=$$(wc -l < benchmarks/results/modest_smc.jsonl); \
	  [ "$$n" -eq 4 ] || { echo "modest: $$n records, expected 4"; exit 1; }; \
	  echo "modest: 4 records"; ran=1; \
	else echo "modest: not found, skipped (set MODEST=)"; fi; \
	if command -v "$(VERIFYTA)" >/dev/null 2>&1; then \
	  rm -f benchmarks/results/uppaal_smc.jsonl; \
	  $(MAKE) bench-uppaal VERIFYTA="$(VERIFYTA)"; \
	  n=$$(wc -l < benchmarks/results/uppaal_smc.jsonl); \
	  want=$$(ls benchmarks/baselines/uppaal/*.xml | wc -l); \
	  [ "$$n" -eq "$$want" ] || { echo "uppaal: $$n records, expected $$want"; exit 1; }; \
	  echo "uppaal: $$n records"; ran=1; \
	else echo "uppaal: not found, skipped (set VERIFYTA=)"; fi; \
	if command -v "$(PRISM)" >/dev/null 2>&1; then \
	  rm -f benchmarks/results/prism_smc.jsonl; \
	  $(MAKE) bench-prism PRISM="$(PRISM)"; \
	  n=$$(wc -l < benchmarks/results/prism_smc.jsonl); \
	  [ "$$n" -eq 2 ] || { echo "prism: $$n records, expected 2"; exit 1; }; \
	  echo "prism: 2 records"; ran=1; \
	else echo "prism: not found, skipped (set PRISM=)"; fi; \
	[ "$$ran" = 1 ] || { echo "no baseline tool was found, nothing refreshed"; exit 1; }; \
	python3 scripts/check_claims.py