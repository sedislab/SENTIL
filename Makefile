# Convenience wrapper; the C ABI recipes live in sentil-ffi/Makefile and the
# embedded host tests in sentil-arduino/Makefile.
.PHONY: build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean test-arduino

build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean:
	$(MAKE) -C sentil-ffi $@

test-arduino:
	$(MAKE) -C sentil-arduino test