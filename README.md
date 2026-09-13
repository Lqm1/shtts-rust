# SHTTS

[![Build](https://github.com/Lqm1/shtts-rust/actions/workflows/build.yml/badge.svg)](https://github.com/Lqm1/shtts-rust/actions/workflows/build.yml)

Deterministic Japanese speech synthesis in Rust, available as a WebAssembly package for JavaScript applications.

[Try the demo](https://lqm1.github.io/shtts-rust/) · [npm package](https://www.npmjs.com/package/shtts-wasm) · [Usage](#javascript-usage) · [Development](#local-development)

## Overview

- Fixed-point synthesis with repeatable PCM output.
- Twelve voice presets, sixteen emotion presets, and adjustable synthesis parameters.
- Mono signed 16-bit PCM at 11,025 Hz.
- A browser demo that generates audio locally in a Web Worker.
- One WASM package shared by npm releases and the GitHub Pages demo.

> [!NOTE]
> The engine accepts Japanese katakana. It skips unknown characters and does not include a kanji reading dictionary or English text-to-speech. The demo also accepts hiragana and converts it to katakana.

## JavaScript usage

```sh
npm install shtts-wasm
```

### Browser with Vite

```js
import init, { Settings, Voice, Emotion, synthesize, sampleRate } from 'shtts-wasm';
import wasmUrl from 'shtts-wasm/shtts_bg.wasm?url';

await init({ module_or_path: wasmUrl });
const settings = Settings.preset(Voice.Female, Emotion.Happy);
try {
  // Katakana for "hello".
  const pcm = synthesize('\u30b3\u30f3\u30cb\u30c1\u30cf', settings);
  console.log(pcm, sampleRate());
} finally {
  settings.free();
}
```

The returned `Int16Array` owns its samples and remains valid after `settings.free()`. Audio playback and encoding belong to the application. See [the demo worker](demo/src/worker.ts) and [WAV encoder](demo/src/wav.ts). Run longer synthesis requests in a Worker to keep the interface responsive.

Without a bundler, serve the generated package files together over HTTP, import `shtts.js`, and call `await init()` to load the adjacent WASM file.

### Node.js and Bun

The package uses ES modules. Supply the WASM bytes explicitly because the default browser loader fetches a URL, including a local file URL when imported from disk.

```js
import { readFile } from 'node:fs/promises';
import init, { Settings, synthesize } from 'shtts-wasm';

const bytes = await readFile(new URL(import.meta.resolve('shtts-wasm/shtts_bg.wasm')));
await init({ module_or_path: bytes });
const settings = new Settings();
try {
  console.log(synthesize('\u30b3\u30f3\u30cb\u30c1\u30cf', settings));
} finally {
  settings.free();
}
```

This loading path has been checked locally in Node.js 24 and Bun. Other runtimes, including Deno, can initialize the same binary with `init({ module_or_path: bytes })` if they support its WebAssembly features. Deno has not been verified in this project; configure npm resolution and file permissions for your environment.

### API at a glance

| Export | Purpose |
| --- | --- |
| `init(options)` | Load and instantiate the WASM module asynchronously. |
| `initSync({ module })` | Initialize from bytes or a compiled `WebAssembly.Module` synchronously. |
| `new Settings()` | Create neutral synthesis settings. |
| `Settings.preset(voice, emotion?)` | Apply a voice and optional emotion preset. |
| `synthesize(text, settings)` | Return mono `Int16Array` samples. |
| `sampleRate()` | Return the playback rate, 11,025 Hz. |
| `Voice`, `Emotion` | Select typed presets. |

Settings expose pitch, accent, phrase accent, volume, speed, spectral scaling, fluctuation, echo, and ring modulation controls. Invalid values throw JavaScript errors. See the generated TypeScript declarations shipped with the package for ranges and parameter meanings.

## Rust usage

The core crate has no external dependencies and forbids unsafe code. Use it from this workspace or as a path dependency.

```rust
use shtts::{synthesize, Voice, VoiceSettings};

fn main() -> Result<(), shtts::InvalidParameter> {
    let settings = VoiceSettings::from(Voice::Female);
    let pcm = synthesize("\u{30b3}\u{30f3}\u{30cb}\u{30c1}\u{30cf}", &settings)?;
    assert!(!pcm.is_empty());
    Ok(())
}
```

The [file output example](examples/synthesize.rs) writes raw little-endian PCM. It does not add a WAV header.

## Local development

Use Rust 1.88 or newer, wasm-pack 0.15.0, Node.js 24, and npm 11. Install the `wasm32-unknown-unknown` Rust target before building WASM.

```sh
git clone https://github.com/Lqm1/shtts-rust.git
cd shtts-rust/demo
npm ci
npm run dev
```

The React + TypeScript demo was scaffolded with `vp create vite` using the `react-ts` template. It uses Vite+ for development, formatting, linting, and bundling. The published npm version is pinned in `demo/package.json`. `App.tsx` contains the UI and state; a Worker handles speech synthesis. Run `npm run check` for formatting, linting, and type checks. Demo-specific ignore rules live in `demo/.gitignore`.

To try local Rust changes, run these commands from `demo/`:

```sh
npm run build:wasm
npm run pack:wasm
npm install --no-save --package-lock=false --ignore-scripts ../target/npm/shtts-wasm-0.1.0.tgz
npm run dev
```

Use the tarball filename printed by `pack:wasm` when the package version changes. After Rust changes, rebuild, repack, reinstall the tarball, and restart Vite. For a production preview, run `npm run build` and `npm run preview`, then open `/shtts-rust/` on the preview server.

| Location | Contents |
| --- | --- |
| `src/` | Text analysis, prosody, acoustic processing, and synthesis. |
| `bindings/web/` | wasm-bindgen bindings and the npm release version. |
| `demo/` | Vite app and its npm tooling. |
| `scripts/package.mjs` | npm metadata, release version validation, and tarball creation. |
| `tests/` | Rust regression tests and reference fixtures. |
| `target/web/`, `target/npm/` | Generated package files and tarballs, excluded from Git. |

Run checks from the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --release --locked
```

CI runs these Rust checks, checks the demo's formatting, linting, and types, and builds the WASM package and demo. There are no automated demo tests.

## Release workflow

The version in `bindings/web/Cargo.toml` controls the npm release. Update it and `Cargo.lock`, commit, then push a matching `vX.Y.Z` tag. Only stable semantic versions are supported.

1. Run the Rust checks and build WASM once.
2. Pack the npm tarball and build the demo from that exact package.
3. Publish through GitHub Actions OIDC, without a stored npm token.
4. Compare the registry integrity with the packed artifact.
5. Deploy the built demo to GitHub Pages.

Ordinary pushes validate changes without updating the public demo. The demo displays its package version. If publication or deployment fails, re-run the failed jobs from the same workflow run. An existing npm version is accepted only if its integrity matches the artifact. npm and Pages cannot update atomically; a failed Pages deployment may temporarily leave the previous demo online.

The npm trusted publisher must authorize GitHub owner `Lqm1`, repository `shtts-rust`, workflow `release.yml`, environment `npm`, and direct publishing. GitHub Pages must use GitHub Actions as its source. Initial package registration requires a one-time authenticated publish before configuring this trust relationship. See [npm's trusted publishing documentation](https://docs.npmjs.com/trusted-publishers/) for account setup.
