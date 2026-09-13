# SHTTS

Deterministic, fixed-point speech synthesis written in Rust and distributed as WebAssembly.

[Browser demo](https://Lqm1.github.io/shtts-rust/) · [npm package](https://www.npmjs.com/package/shtts)

Input is katakana, not arbitrary kanji text. Unknown characters are skipped. The demo converts hiragana to katakana before synthesis. Synthesis returns mono signed 16-bit PCM at 11,025 Hz. Audio playback is the application's responsibility.

## Browser with Vite

```js
import init, { Settings, Voice, synthesize } from 'shtts';
import wasmUrl from 'shtts/shtts_bg.wasm?url';

await init({ module_or_path: wasmUrl });
const settings = Settings.preset(Voice.Female);
try {
  const pcm = synthesize('コンニチハ。', settings);
  console.log(pcm);
} finally {
  settings.free();
}
```

Use a Web Worker for longer inputs. See `demo/src/worker.js` for WAV encoding and worker usage. Without a bundler, serve the package files together and import `shtts.js`; `await init()` loads the adjacent WASM file over HTTP.

## Node.js and Bun

Install with `npm install shtts`. This package is ESM. The same WASM binary works outside browsers; explicitly supply its bytes rather than relying on fetching a local file URL.

```js
import { readFile } from 'node:fs/promises';
import init, { Settings, synthesize } from 'shtts';

await init({ module_or_path: await readFile(new URL(import.meta.resolve('shtts/shtts_bg.wasm'))) });
const settings = new Settings();
try {
  console.log(synthesize('コンニチハ。', settings));
} finally {
  settings.free();
}
```

Deno can also instantiate the binary by passing a `Uint8Array` to `init({ module_or_path: bytes })`. File access and npm resolution must be configured for that environment. Runtime compatibility depends on the WebAssembly features supported by the runtime, not just the build target name.

## Development

Requires Rust, wasm-pack 0.15.0, Node.js 24 and npm 11.

```sh
cd demo
npm ci
npm run build:wasm
npm run pack:wasm
npm install --no-save --package-lock=false --ignore-scripts ../target/npm/shtts-0.1.0.tgz
npm run test:package
npm run dev
```

Run `npm run build` followed by `npm run preview` and open the displayed `/shtts-rust/` URL to verify the production demo. CI checks the installed package in Node.js and builds the demo from that same package.

## Releases

The version in `bindings/web/Cargo.toml` controls the npm version. Update it and Cargo.lock, commit, then push a matching `vX.Y.Z` tag. The release workflow tests and packs once, installs that tarball into the demo, publishes it using GitHub Actions OIDC, checks the registry integrity, and deploys the built site. It does not use an npm token. Ordinary pushes only validate changes; they do not update the public demo.

For a failed release, re-run failed jobs on the same workflow run. An already-published version is accepted only when its integrity matches the build artifact. npm and Pages cannot be updated atomically; if Pages fails after npm publication, re-run deployment. Do not overwrite an existing version or move a published tag.

Initial setup: publish a working `0.0.0` bootstrap package with the `bootstrap` dist-tag from an authenticated terminal. Configure the npm trusted publisher for GitHub user `Lqm1`, repository `shtts-rust`, workflow `release.yml`, environment `npm`, and allow direct publishing. Enable GitHub Pages with GitHub Actions as its source. Subsequent releases use OIDC and automatically generated provenance.

Licensed under Apache-2.0. See [LICENSE](LICENSE).
