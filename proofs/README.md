# Proofs

A Lean proof that asserts that the monotonic deque that we implement in [`sentil-core/src/semantics/window.rs`](../sentil-core/src/semantics/window.rs) computes the same answer as a naive scan.

## What exactly we prove

For a stream of timestamped values processed in time order, `deque_sliding_window_min_correct` states that after the k-th sample the deque's front value equals the minimum value over the window `[t_k - w, t_k]` inclusive. `deque_sliding_window_max_correct` also proves that after the k-th sample the deque's front value equals the maximum value over the window `[t_k - w, t_k]` inclusive.

The deque keeps timestamps strictly increasing from front to back and values non-decreasing from front to back. It evicts from the front while the front timestamp falls strictly below `t_k - w`, and evicts from the back while the back value is at least the incoming value.

## Verifying it

The toolchain is pinned in `lean-toolchain` (Lean 4.31.0). With [elan](https://github.com/leanprover/elan) installed, run

```bash
cd proofs
lake build
```