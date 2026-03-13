# Convenience wrapper; the C ABI recipes live in sentil-ffi/Makefile and the
# embedded host tests in sentil-embedded/Makefile.
.PHONY: build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean test-embedded verify bench-modest

MODEST ?= modest

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
	@tmp=$$(mktemp); \
	for m in benchmarks/baselines/modest/*.modest; do \
	  MODEST=$(MODEST) bash benchmarks/runners/modest_runner.sh $$m >>$$tmp || exit 1; \
	done; \
	if [ -s $$tmp ]; then mv $$tmp benchmarks/results/modest_smc.jsonl; else rm -f $$tmp; fi