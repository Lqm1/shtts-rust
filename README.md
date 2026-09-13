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

For UI-only updates, run the **Deploy demo** workflow manually on master. It uses the published npm version pinned in demo/package-lock.json without rebuilding or publishing WASM.

## Research purpose and reproduction method

This project is an independent Rust reimplementation of the SHTTS speech synthesis technology used in some Nintendo DS and Nintendo 3DS games, including titles in the Tomodachi Collection series. It was developed to study and reproduce the behavior of that technology in a lightweight implementation, with embedded use as a design goal.

Reference audio was collected by running the original games in an emulator with a wide range of inputs and settings. The implementation was refined through repeated comparison and trial and error until its output matched the reference PCM sample for sample. For the covered reference cases, this means identical sample values and lengths, rather than merely similar-sounding speech.

The reference set covers a broad range of patterns, but it is not exhaustive. Untested inputs, settings, or game-specific variations may still produce differences. Exact matches on the covered cases should not be interpreted as a guarantee of complete compatibility with every game or every possible input.

## Related projects

- [dylanpdx/talkmodachi](https://github.com/dylanpdx/talkmodachi) inspired the concept. It exposes Tomodachi Life's speech synthesis through a patched game and a custom build of the Citra emulator.
- [26d0/shtts-web](https://github.com/26d0/shtts-web) provides an example of a lightweight emulation approach, running the SHTTS engine from Shaberu! DS Oryouri Navi in a browser using a supplied ROM or extracted ARM9 binary.

Both approaches depend on original game data and execute the original engine through emulation. Avoiding that runtime dependency and its processing overhead motivated this Rust reimplementation. The goal is a small, efficient synthesizer that runs without loading a game ROM or running an emulator, including in resource-constrained applications. This is a design goal, not a claim of benchmarked performance against those projects or support for every embedded target.

## Disclaimer

This is an unofficial research project. It is not affiliated with, endorsed by, or sponsored by Nintendo, SHARP, or the developers and rights holders of the original SHTTS technology or the games mentioned here. Product names and trademarks are used only to identify the systems being studied and remain the property of their respective owners.

The software is provided as is, without guarantees of complete compatibility, accuracy for untested cases, or suitability for a particular application. The research purpose described here does not grant rights to third-party software, game data, trademarks, or other protected material. The repository's license applies only to material that its contributors are entitled to license.

## Rights-holder removal requests

Removal requests are considered only when submitted by a legitimate rights holder in the original SHTTS technology or project, or by a representative authorized to act on that rights holder's behalf. Requests from unrelated third parties are not accepted as rights-holder removal requests.

To submit a request, open an [issue in this repository](https://github.com/Lqm1/shtts-rust/issues) identifying:

- The rights holder and your relationship to them.
- The specific files, content, or distribution you are requesting be removed.
- The rights involved and the basis for the request.
- A way to verify your authority and contact you for follow-up.

Do not post confidential documents or personal identification in a public issue. If private verification is necessary, ask the maintainer to arrange an appropriate contact channel. Verified requests will be reviewed in good faith, and the relevant material will be removed or otherwise addressed as appropriate.
