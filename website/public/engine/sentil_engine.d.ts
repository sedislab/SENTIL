/* tslint:disable */
/* eslint-disable */

/**
 * A streaming monitor that folds one timestamped sample at a time. Values are
 * passed in the variable order reported by `parse_formula`.
 */
export class StreamMonitor {
    free(): void;
    [Symbol.dispose](): void;
    constructor(src: string);
    reset(): void;
    update(time: number, values: Float64Array): string;
}

/**
 * Estimate satisfaction probability of a PrSTL property over one noisy channel,
 * with its confidence interval.
 */
export function check_prstl(req_json: string): string;

/**
 * Parse a formula and report its variables, or the parse error.
 */
export function parse_formula(src: string): string;

/**
 * Evaluate robustness over a whole trace: the scalar, the per-sample series, and
 * the violation intervals. `dense` selects interpolated dense-time semantics.
 */
export function robustness(req_json: string): string;

/**
 * The Wilson score interval, a pure closed form with no sampling, so the docs
 * can animate an interval shrinking as the trial count grows.
 */
export function wilson(successes: number, trials: number, level: number): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_streammonitor_free: (a: number, b: number) => void;
    readonly check_prstl: (a: number, b: number) => [number, number];
    readonly parse_formula: (a: number, b: number) => [number, number];
    readonly robustness: (a: number, b: number) => [number, number];
    readonly streammonitor_new: (a: number, b: number) => [number, number, number];
    readonly streammonitor_reset: (a: number) => void;
    readonly streammonitor_update: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wilson: (a: number, b: number, c: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
