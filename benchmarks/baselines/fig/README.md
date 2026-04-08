# FIG baseline for the rare-event benchmarks

[FIG, the Finite Improbability Generator](https://git.cs.famaf.unc.edu.ar/dsg/fig), is a dedicated rare-event simulator that estimates reachability by importance splitting (RESTART and fixed effort) on IOSA models.

## The model

`models/tandem_queue.sa` models two queues in tandem, arrivals at rate 3, queue one draining into queue two at rate 2, queue two draining at rate 6, both bounded at capacity `c`. The transient property is `P(q2 > 0 U q2 == c)` and it checks that from one packet in queue two, it overflows to `c` before it empties. SENTIL runs the same continuous-time chain through its embedded jump chain in `sentil_rare_tandem_runner`. The overflow probability at c=8 is 5.60e-6, at c=10 is 3.15e-7, at c=12 is 1.86e-8.

## Building FIG

FIG is GPL and depends on the z3 SMT library.

```
# z3: point FIND_LIBRARY and the header path at any libz3 (the pip z3-solver ships one)
# jsoncpp (vendored, 2016): force-include <cstdint> for the uintN_t types GCC 13 no longer pulls in transitively
# a __gnu_cxx::new_allocator friend hack: build with the gcc it targets (gcc 5.4 .. 10), not gcc 13
cmake -DRELEASE=ON -DZ3=/path/to/libz3.so \
      -DCMAKE_C_COMPILER=gcc-10 -DCMAKE_CXX_COMPILER=g++-10 \
      -DCMAKE_CXX_FLAGS="-I/path/to/z3/include -include cstdint" ..
make -j
```

## Running it

```
./fig models/tandem_queue.sa --adhoc q2 -e restart -t es --stop-conf 0.9 0.2
```

`--adhoc q2` uses the queue length as the importance function, `-e restart` the RESTART engine, `-t es` the expected-success threshold builder, and `--stop-conf 0.9 0.2` stops at a 90 percent confidence interval of 20 percent relative half-width. The measured numbers are in `results/fig_rare_event.jsonl`. On the same model and confidence, SENTIL reaches the same precision in milliseconds.