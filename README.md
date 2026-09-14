# SHTTS

[![Build](https://github.com/Lqm1/shtts-rust/actions/workflows/build.yml/badge.svg)](https://github.com/Lqm1/shtts-rust/actions/workflows/build.yml)

An independent Rust reimplementation of the SHTTS speech synthesis technology used in some Nintendo DS and Nintendo 3DS games, including titles in the Tomodachi Collection series. Developed for research into its behavior, it provides deterministic Japanese speech synthesis as a native Rust library and a WebAssembly package.

The goal is a lightweight implementation suitable for resource-constrained and embedded applications, without loading a game ROM or running an emulator at runtime.

[Try the demo](https://lqm1.github.io/shtts-rust/) · [npm package](https://www.npmjs.com/package/shtts-wasm) · [Usage](#javascript-usage) · [Development](#local-development)

## Overview

- Fixed-point synthesis with repeatable PCM output.
- Twelve voice presets, sixteen emotion presets, and adjustable synthesis parameters.
- Mono signed 16-bit PCM at 11,025 Hz.
- A browser demo that generates audio locally in a Web Worker.
- One WASM package shared by npm releases and the GitHub Pages demo.

> [!NOTE]
> The engine accepts Japanese katakana. It skips unknown characters and does not include a kanji reading dictionary or English text-to-speech. The demo also accepts hiragana and converts it to katakana.

The implementation was refined through trial and error against audio captured from original games running in an emulator. Its output matches the covered reference cases exactly, including every PCM sample and the output length. The reference set covers a broad range of inputs and settings, but untested patterns and game-specific variations may still differ.

## JavaScript usage

```sh
npm install shtts-wasm
```

### Browser with Vite 8.1 or newer

```js
import { Settings, Voice, Emotion, synthesize, sampleRate } from 'shtts-wasm/bundler';

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

[Vite 8.1 added native WASM ESM integration](https://vite.dev/blog/announcing-vite8-1), so the `bundler` target initializes automatically without a WASM plugin. The demo uses this target in its module Worker. The UI imports only `Voice` and `Emotion` from `web`, which avoids initializing another WASM instance on the main thread. Direct WASM imports require top-level await support; the demo emits ES module workers with `worker: { format: 'es' }`.

For older Vite versions without WASM ESM integration, or when explicit initialization is needed, use the `web` target with Vite's asset URL handling:

```js
import init, { Settings, synthesize } from 'shtts-wasm/web';
import wasmUrl from 'shtts-wasm/web/shtts_bg.wasm?url';

await init({ module_or_path: wasmUrl });
```

Without a bundler, serve the package files together over HTTP, import `web/shtts.js`, and call `await init()` to load the adjacent WASM file. Always choose a target subpath; the package has no root import entry.

### Node.js and Bun

The Node.js target loads its adjacent WASM file synchronously. It works with CommonJS `require` and with imports in Node.js and Bun; no explicit initialization is needed.

```js
import { Settings, synthesize } from 'shtts-wasm/nodejs';

const settings = new Settings();
try {
  console.log(synthesize('\u30b3\u30f3\u30cb\u30c1\u30cf', settings));
} finally {
  settings.free();
}
```

For CommonJS, use `const { Settings, synthesize } = require('shtts-wasm/nodejs')`.

### Available targets

Every target in [wasm-bindgen's deployment guide](https://wasm-bindgen.github.io/wasm-bindgen/reference/deployment.html) is included, with its generated JavaScript, TypeScript declarations, and WASM file.

| Package subpath | Loading behavior |
| --- | --- |
| `shtts-wasm/web` | Browser ES module with explicit initialization. Call `await init()`; with Vite, pass the explicit `?url` asset as above. |
| `shtts-wasm/nodejs` | CommonJS with synchronous initialization. Suitable for Node.js and Bun. |
| `shtts-wasm/bundler` | WASM ES module imports with automatic initialization. Recommended for Vite 8.1+; also supported by webpack with `experiments.asyncWebAssembly: true`. Older Vite versions need a WASM plugin or the `web` target. |
| `shtts-wasm/deno` | ES module with top-level asynchronous initialization. In Deno, import from `npm:shtts-wasm/deno`; allow reads for the adjacent WASM file. |
| `shtts-wasm/no-modules` | Classic script exposing `wasm_bindgen`. Load `no-modules/shtts.js` with a script tag, then call `await wasm_bindgen({ module_or_path: './no-modules/shtts_bg.wasm' })`. This target is not an ES module and cannot provide named imports. |
| `shtts-wasm/experimental-nodejs-module` | Node.js ES module with synchronous initialization. The upstream target is experimental. |
| `shtts-wasm/module` | Source phase WASM imports with automatic initialization. Requires runtime or bundler support for `import source`; Node.js 24 uses `--experimental-wasm-modules`. |

The `shtts-wasm/<target>/shtts.js`, `shtts.d.ts`, and `shtts_bg.wasm` files are also exported under each target directory. Keep each generated directory intact when serving files directly.

Rust is compiled once, then wasm-bindgen generates each target separately. We retain each target's WASM rather than rewriting generated loaders to share one file. The current optimized binaries are byte-identical, but sharing would couple packaging to generated loader syntax and future target differences. Separate files cost package size and preserve the official outputs without custom runtime code.

### API at a glance

| Export | Purpose |
| --- | --- |
| `init(options)` | Load and instantiate WASM asynchronously on the web target. |
| `initSync({ module })` | Initialize the web target from bytes or a compiled `WebAssembly.Module` synchronously. |
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

Use Rust 1.88 or newer, wasm-bindgen-cli 0.2.120, Binaryen's `wasm-opt`, Node.js 24, and npm 11. Install the `wasm32-unknown-unknown` Rust target before building WASM. Install the matching CLI with `cargo install wasm-bindgen-cli --version 0.2.120 --locked`. Put `wasm-opt` on PATH or set `WASM_OPT` to its executable path. The build script checks the CLI version against the binding crate's pinned dependency.

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
npm install --no-save --package-lock=false --ignore-scripts ../target/npm/shtts-wasm-0.2.0.tgz
npm run dev
```

Use the tarball filename printed by `pack:wasm` when the package version changes. After Rust changes, rebuild, repack, reinstall the tarball, and restart Vite. For a production preview, run `npm run build` and `npm run preview`, then open `/shtts-rust/` on the preview server.

| Location | Contents |
| --- | --- |
| `src/` | Text analysis, prosody, acoustic processing, and synthesis. |
| `bindings/wasm/` | wasm-bindgen bindings and the npm release version. |
| `demo/` | Vite app and its npm tooling. |
| `scripts/package.mjs` | npm metadata, release version validation, and tarball creation. |
| `scripts/build-wasm.mjs` | One Cargo build, then binding generation and WASM optimization for all seven targets. |
| `tests/` | Rust regression tests and reference fixtures. |
| `target/package/`, `target/npm/` | Generated package files and tarballs, excluded from Git. |

Run checks from the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --release --locked
```

CI runs these Rust checks, checks the demo's formatting, linting, and types, and builds all WASM targets and the demo from the packed tarball. The Vite demo build also validates asset bundling. There are no automated browser UI tests.

## Release workflow

The version in `bindings/wasm/Cargo.toml` controls the npm release. Update it and `Cargo.lock`, commit, then push a matching `vX.Y.Z` tag. Only stable semantic versions are supported.

1. Run the Rust checks and build WASM once.
2. Pack the npm tarball and build the demo from that exact package.
3. Publish through GitHub Actions OIDC, without a stored npm token.
4. Compare the registry integrity with the packed artifact.
5. Deploy the built demo to GitHub Pages.

Ordinary pushes validate changes without updating the public demo. The demo displays its package version. If publication or deployment fails, re-run the failed jobs from the same workflow run. An existing npm version is accepted only if its integrity matches the artifact. npm and Pages cannot update atomically; a failed Pages deployment may temporarily leave the previous demo online.

The npm trusted publisher must authorize GitHub owner `Lqm1`, repository `shtts-rust`, workflow `release.yml`, environment `npm`, and direct publishing. GitHub Pages must use GitHub Actions as its source. Initial package registration requires a one-time authenticated publish before configuring this trust relationship. See [npm's trusted publishing documentation](https://docs.npmjs.com/trusted-publishers/) for account setup.

For UI-only updates, run the **Deploy demo** workflow manually on master. It uses the published npm version pinned in demo/package-lock.json without rebuilding or publishing WASM.

## Related projects

- [dylanpdx/talkmodachi](https://github.com/dylanpdx/talkmodachi) inspired the concept. It uses a patched version of Tomodachi Life and a custom Citra build to expose the game's speech synthesis.
- [26d0/shtts-web](https://github.com/26d0/shtts-web) demonstrates a lightweight emulation approach, running the SHTTS engine from Shaberu! DS Oryouri Navi in a browser using a supplied ROM or extracted ARM9 binary.

Both rely on original game data and emulation. Removing those runtime requirements and reducing their processing overhead motivated this Rust reimplementation; comparative performance and support for individual embedded targets have not been established.

## Project status and rights

This is an unofficial research project, with no affiliation, sponsorship, or endorsement from Nintendo, SHARP, or the original SHTTS developers or rights holders. Product names and trademarks identify the systems being studied and belong to their respective owners. The software is provided as is, without warranties of compatibility or suitability. Its research purpose and repository license do not grant rights to third-party software or game data.

Removal requests are accepted only from legitimate rights holders in the original SHTTS technology or project, or their authorized representatives. Please open an [issue](https://github.com/Lqm1/shtts-rust/issues) identifying the rights holder, the material concerned, the basis for the request, and a way to verify your authority. Do not post confidential documents or personal identification publicly; ask for a private contact channel if needed. Verified requests will be reviewed in good faith, with the relevant material removed or otherwise addressed as appropriate.
