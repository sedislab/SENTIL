CARGO ?= cargo
CXX ?= c++
CXXFLAGS ?= -std=c++14 -Wall -Wextra -O2

VALGRIND ?= valgrind
ROOT := $(abspath ..)
RUST := rust
LIB := $(RUST)/target/release/libsentil_embedded.a
COMPILER := $(RUST)/target/release/sentil-compile-formula
DLIB := $(RUST)/target/debug/libsentil_embedded.a
DCOMPILER := $(RUST)/target/debug/sentil-compile-formula
ORACLE := $(ROOT)/benchmarks/deterministic/oracle.json
BLOB := $(abspath $(RUST)/target/sentil_test_formula.bin)
LINK := $(LIB) -lpthread -ldl -lm
TESTS := tests/test_oracle.cpp tests/test_monitor.cpp tests/test_synthesis.cpp
DEFS = -DSENTIL_ORACLE_PATH='"$(ORACLE)"' -DSENTIL_BLOB_PATH='"$(BLOB)"'
VG := $(VALGRIND) --leak-check=full --show-leak-kinds=definite,indirect \
	--errors-for-leak-kinds=definite,indirect --error-exitcode=1 --quiet

.PHONY: build test test-rust leakcheck clean

build:
	$(CARGO) build --release --manifest-path $(RUST)/Cargo.toml --features std

test: build
	$(COMPILER) "always[0, 5](x > 0)" -o $(BLOB)
	@for t in $(TESTS); do \
		echo "  cxx $$t"; out=$$(mktemp); \
		$(CXX) $(CXXFLAGS) -Iinclude -Itests $(DEFS) \
			$$t src/Sentil.cpp $(LINK) -o $$out || exit 1; \
		$$out || exit 1; rm -f $$out; \
	done
	@echo "all embedded host tests passed"

test-rust:
	$(CARGO) test --manifest-path $(RUST)/Cargo.toml --features std

leakcheck:
	$(CARGO) build --manifest-path $(RUST)/Cargo.toml --features std
	$(DCOMPILER) "always[0, 5](x > 0)" -o $(BLOB)
	@for t in $(TESTS); do \
		echo "  valgrind $$t"; out=$$(mktemp); \
		$(CXX) -std=c++14 -g -O0 -Iinclude -Itests $(DEFS) \
			$$t src/Sentil.cpp $(DLIB) -lpthread -ldl -lm -o $$out || exit 1; \
		$(VG) $$out || exit 1; rm -f $$out; \
	done
	@echo "no definite or indirect leaks"

clean:
	$(CARGO) clean --manifest-path $(RUST)/Cargo.toml