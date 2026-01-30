# Proofs

A Lean 4 formalization of the one algorithm the streaming monitor's speed rests on: the monotonic deque that answers a sliding-window minimum in amortized O(1) per sample. `MonotonicDeque.lean` proves it computes the same answer as the naive scan it replaces, so the fast path in [`sentil-core/src/semantics/window.rs`](../sentil-core/src/semantics/window.rs) is trusted rather than merely tested.

## What is proven

For a stream of timestamped values processed in time order, `deque_sliding_window_min_correct` states that after the k-th sample the deque's front value equals the minimum value over the window `[t_k - w, t_k]` inclusive. `deque_sliding_window_max_correct` is the dual, for the eventually operator's supremum.

The deque keeps timestamps strictly increasing from front to back and values non-decreasing from front to back. It evicts from the front while the front timestamp falls strictly below `t_k - w`, and evicts from the back while the back value is at least the incoming value. That back rule is `>=`, matching `dominated` in window.rs, so a run of equal values keeps only the newest; the minimum is the same either way, and `popBack_val_lt` proves the surviving tail stays strictly below the value being pushed.

## Verifying it

The toolchain is pinned in `lean-toolchain` (Lean 4.31.0). With [elan](https://github.com/leanprover/elan) installed:

```
cd proofs
lake build
```

The `#eval` lines run the executable deque against the naive scan on a handful of streams and print `PASS`.