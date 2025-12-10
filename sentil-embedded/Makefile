CARGO ?= cargo
CXX ?= c++
CXXFLAGS ?= -std=c++14 -Wall -Wextra -O2

ROOT := $(abspath ..)
RUST := rust
LIB := $(RUST)/target/release/libsentil_embedded.a
COMPILER := $(RUST)/target/release/sentil-compile-formula
ORACLE := $(ROOT)/benchmarks/deterministic/oracle.json
BLOB := $(abspath $(RUST)/target/sentil_test_formula.bin)
LINK := $(LIB) -lpthread -ldl -lm
TESTS := tests/test_oracle.cpp tests/test_monitor.cpp tests/test_synthesis.cpp

.PHONY: build test clean

build:
	$(CARGO) build --release --manifest-path $(RUST)/Cargo.toml --features std

test: build
	$(COMPILER) "always[0, 5](x > 0)" -o $(BLOB)
	@for t in $(TESTS); do \
		echo "  cxx $$t"; out=$$(mktemp); \
		$(CXX) $(CXXFLAGS) -Iinclude -Itests \
			-DSENTIL_ORACLE_PATH='"$(ORACLE)"' -DSENTIL_BLOB_PATH='"$(BLOB)"' \
			$$t src/Sentil.cpp $(LINK) -o $$out || exit 1; \
		$$out || exit 1; rm -f $$out; \
	done
	@echo "all embedded host tests passed"

clean:
	$(CARGO) clean --manifest-path $(RUST)/Cargo.toml