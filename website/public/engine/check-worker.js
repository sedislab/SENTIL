import init, { parse_formula, robustness, wilson } from './sentil_engine.js';

let boot = null;

function ensure() {
  boot ??= init({ module_or_path: new URL('./sentil_engine_bg.wasm', import.meta.url) });
  return boot;
}

self.onmessage = async (event) => {
  const { id, op, formula, trace } = event.data;
  if (op === 'warm') {
    ensure().catch(() => {});
    return;
  }
  try {
    await ensure();
    if (op === 'parse') {
      self.postMessage({ id, result: JSON.parse(parse_formula(formula)) });
      return;
    }
    if (op === 'wilson') {
      const { successes, trials, level } = event.data;
      self.postMessage({ id, result: { ok: true, ...JSON.parse(wilson(successes, trials, level)) } });
      return;
    }
    self.postMessage({
      id,
      result: JSON.parse(
        robustness(
          JSON.stringify({
            formula,
            times: trace.times,
            signals: trace.signals,
            dense: trace.dense ?? false,
          }),
        ),
      ),
    });
  } catch (err) {
    self.postMessage({ id, result: { ok: false, error: String((err && err.message) || err) } });
  }
};