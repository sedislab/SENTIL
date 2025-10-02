# Convenience wrapper; the recipes live in sentil-ffi/Makefile.
.PHONY: build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean

build test-ffi test-ffi-gpu leakcheck bench-c bench-py clean:
	$(MAKE) -C sentil-ffi $@