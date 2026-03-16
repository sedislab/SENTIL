# Convenience wrapper; the C ABI recipes live in sentil-ffi/Makefile and the
# embedded host tests in sentil-embedded/Makefile.
.PHONY: build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean test-embedded verify bench-modest bench-uppaal

MODEST ?= modest
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
	VERIFYTA="$(VERIFYTA)" bash benchmarks/runners/uppaal_runner.sh \
	  benchmarks/baselines/uppaal/circadian.xml >>"$$tmp" || exit 1; \
	if [ -s "$$tmp" ]; then cp "$$tmp" benchmarks/results/uppaal_smc.jsonl; fi