# SENTIL documentation site

The SENTIL documentation, built with [Fumadocs](https://fumadocs.dev) on Next.js and exported to a static site. Content lives in `content/docs` as MDX; the browser playground runs the real SENTIL engine compiled to WebAssembly from `wasm/`.

## Develop

```bash
npm install
npm run dev
```

The site is a static export (`output: 'export'` in `next.config.mjs`), so `npm run build` writes a fully static `out/` that any CDN can serve. `public/_headers` carries the content-security policy and the rest of the security headers for Cloudflare Pages.

## The browser engine

`wasm/` is a small wrapper crate over `sentil-core`, built to `wasm32-unknown-unknown` and turned into browser glue with `wasm-bindgen --target web`. The output sits in `public/engine/` and is loaded by `public/engine/check-worker.js` inside a web worker, so the docs pages carry none of the wasm cost until a visitor opens the playground. The Python playground runs on a second worker, `public/engine/py-worker.js`.

To rebuild the engine after a core change:

```bash
cd wasm
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir ../public/engine \
  target/wasm32-unknown-unknown/release/sentil_engine.wasm
```