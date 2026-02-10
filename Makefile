# Convenience wrapper; the C ABI recipes live in sentil-ffi/Makefile and the
# embedded host tests in sentil-embedded/Makefile.
.PHONY: build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean test-embedded verify

build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean:
	$(MAKE) -C sentil-ffi $@

test-embedded:
	$(MAKE) -C sentil-embedded test

verify:
	python3 scripts/check_version.py
	cargo test --offline -p sentil
	$(MAKE) -C sentil-ffi test-ffi CARGO='cargo --offline'
	python3 scripts/check_claims.py