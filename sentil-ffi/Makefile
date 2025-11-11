CARGO ?= cargo
CC ?= cc
CXX ?= c++
CFLAGS ?= -std=c11 -Wall -Wextra -O2
CXXFLAGS ?= -std=c++17 -O2
PREFIX ?= /usr/local

ROOT := $(abspath ..)
LIBDIR := $(ROOT)/target/release
INCLUDE := include
VERSION := 1.0.0
ARCH := $(shell uname -m)
ifeq ($(shell uname -s),Darwin)
	OSNAME := macos
	SHARED := libsentil.dylib
else
	OSNAME := linux
	SHARED := libsentil.so
endif
BUNDLE := sentil-$(VERSION)-$(OSNAME)-$(ARCH)
DIST := $(ROOT)/target/dist
TESTS := $(wildcard tests/*.c)
GPU_TEST := tests/gpu_test.c
CPU_TESTS := $(filter-out $(GPU_TEST),$(TESTS))
LINK := -L$(LIBDIR) -lsentil -Wl,-rpath,$(LIBDIR) -lm

.PHONY: build test-ffi test-ffi-gpu leakcheck bench-c bench-cpp bench-py bench-jl dist install clean

build:
	$(CARGO) build --release -p sentil-ffi

test-ffi: build
	@for t in $(CPU_TESTS); do \
		echo "  cc $$t"; out=$$(mktemp); \
		$(CC) $(CFLAGS) -I$(INCLUDE) -Itests/support $$t $(LINK) -o $$out || exit 1; \
		$$out || exit 1; rm -f $$out; \
	done
	@echo "all C tests passed"

test-ffi-gpu: build
	@out=$$(mktemp); \
	$(CC) $(CFLAGS) -I$(INCLUDE) -Itests/support $(GPU_TEST) $(LINK) -o $$out || exit 1; \
	$$out || exit 1; rm -f $$out

leakcheck: build
	@for t in $(CPU_TESTS); do \
		out=$$(mktemp); \
		$(CC) $(CFLAGS) -I$(INCLUDE) -Itests/support $$t $(LINK) -o $$out || exit 1; \
		valgrind --leak-check=full --errors-for-leak-kinds=definite,indirect \
			--error-exitcode=99 -q $$out || exit 1; \
		rm -f $$out; \
	done
	@echo "no leaks"

bench-c: build
	$(CC) $(CFLAGS) -I$(INCLUDE) ../benchmarks/runners/sentil_runner.c $(LINK) \
		-o $(ROOT)/target/sentil_bench_c

bench-cpp: build
	$(CXX) $(CXXFLAGS) -I$(INCLUDE) -I../sentil-cpp/include \
		../benchmarks/runners/sentil_runner.cpp $(LINK) -o $(ROOT)/target/sentil_bench_cpp

bench-py: build
	cd ../benchmarks && python runners/sentil_runner.py

bench-jl: build
	cd ../benchmarks && SENTIL_LIB=$(LIBDIR)/$(SHARED) julia --project=../sentil-jl runners/sentil_runner.jl scalability
	cd ../benchmarks && SENTIL_LIB=$(LIBDIR)/$(SHARED) julia --project=../sentil-jl runners/sentil_runner.jl streaming

dist: build
	rm -rf $(DIST)/$(BUNDLE)
	mkdir -p $(DIST)/$(BUNDLE)/lib/pkgconfig $(DIST)/$(BUNDLE)/lib/cmake/Sentil \
		$(DIST)/$(BUNDLE)/include
	cp $(LIBDIR)/$(SHARED) $(LIBDIR)/libsentil.a $(DIST)/$(BUNDLE)/lib/
	cp $(INCLUDE)/sentil.h $(DIST)/$(BUNDLE)/include/
	cp README.md $(DIST)/$(BUNDLE)/
	sed 's|@PREFIX@|/usr/local|' sentil.pc.in > $(DIST)/$(BUNDLE)/lib/pkgconfig/sentil.pc
	cp cmake/SentilConfig.cmake.in $(DIST)/$(BUNDLE)/lib/cmake/Sentil/SentilConfig.cmake
	tar -C $(DIST) -czf $(DIST)/$(BUNDLE).tar.gz $(BUNDLE)
	@echo "bundle: $(DIST)/$(BUNDLE).tar.gz"

install: build
	install -d $(PREFIX)/lib $(PREFIX)/include $(PREFIX)/lib/pkgconfig $(PREFIX)/lib/cmake/Sentil
	install -m644 $(LIBDIR)/libsentil.so $(PREFIX)/lib/
	install -m644 $(INCLUDE)/sentil.h $(PREFIX)/include/
	sed 's|@PREFIX@|$(PREFIX)|' sentil.pc.in > $(PREFIX)/lib/pkgconfig/sentil.pc
	install -m644 cmake/SentilConfig.cmake.in $(PREFIX)/lib/cmake/Sentil/SentilConfig.cmake

clean:
	$(CARGO) clean -p sentil-ffi