# Convenience wrapper; the C ABI recipes live in sentil-ffi/Makefile and the
# embedded host tests in sentil-embedded/Makefile.
.PHONY: build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean test-embedded

build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean:
	$(MAKE) -C sentil-ffi $@

test-embedded:
	$(MAKE) -C sentil-embedded test