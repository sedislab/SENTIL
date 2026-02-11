import init, {
  parse_formula,
  robustness,
  check_prstl,
  wilson,
  StreamMonitor,
} from './sentil_engine.js';
import { loadPyodide } from '/pyodide/pyodide.mjs';

let boot = null;

function ensure() {
  boot ??= (async () => {
    const [, shim] = await Promise.all([
      init({ module_or_path: new URL('./sentil_engine_bg.wasm', import.meta.url) }),
      fetch(new URL('./sentil_shim.py', import.meta.url)).then((r) => r.text()),
    ]);
    self.sentil_engine = { parse_formula, robustness, check_prstl, wilson, StreamMonitor };
    const py = await loadPyodide({ indexURL: '/pyodide/' });
    py.FS.writeFile('sentil.py', shim);
    return py;
  })();
  return boot;
}

self.onmessage = async (event) => {
  const { id, code, op } = event.data;
  if (op === 'warm') {
    ensure().catch(() => {});
    return;
  }
  try {
    if (!boot) self.postMessage({ id, status: 'booting' });
    const py = await ensure();
    self.postMessage({ id, status: 'running' });
    py.setStdout({ batched: (text) => self.postMessage({ id, stream: 'stdout', text }) });
    py.setStderr({ batched: (text) => self.postMessage({ id, stream: 'stderr', text }) });
    await py.runPythonAsync(code);
    self.postMessage({ id, done: true });
  } catch (err) {
    self.postMessage({ id, done: true, error: String((err && err.message) || err) });
  }
};