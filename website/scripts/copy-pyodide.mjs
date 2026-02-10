import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const src = dirname(createRequire(import.meta.url).resolve('pyodide/package.json'));
const dest = join(root, 'public', 'pyodide');

mkdirSync(dest, { recursive: true });
for (const file of [
  'pyodide.mjs',
  'pyodide.asm.js',
  'pyodide.asm.wasm',
  'python_stdlib.zip',
  'pyodide-lock.json',
]) {
  copyFileSync(join(src, file), join(dest, file));
}