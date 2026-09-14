import { execFileSync } from 'node:child_process';
import { mkdir, readFile, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const targets = ['web', 'bundler', 'nodejs', 'deno', 'no-modules', 'experimental-nodejs-module', 'module'];

process.chdir(fileURLToPath(new URL('../', import.meta.url)));
const cargo = await readFile('bindings/web/Cargo.toml', 'utf8');
const version = cargo.match(/^wasm-bindgen = "=([^"]+)"/m)[1];
const run = (command, args) => execFileSync(command, args, { stdio: 'inherit' });
if (execFileSync('wasm-bindgen', ['--version'], { encoding: 'utf8' }).trim() !== `wasm-bindgen ${version}`) {
  throw new Error(`Install the matching CLI: cargo install wasm-bindgen-cli --version ${version} --locked`);
}
const optimizer = process.env.WASM_OPT || 'wasm-opt';
run(optimizer, ['--version']);
run('cargo', ['build', '-p', 'shtts-web', '--target', 'wasm32-unknown-unknown', '--release', '--locked', '--target-dir', 'target']);
// Only remove this script's generated output, never the Cargo build directory.
await rm('target/package', { recursive: true, force: true });
await mkdir('target/package', { recursive: true });
for (const target of targets) {
  const directory = `target/package/${target}`;
  run('wasm-bindgen', ['target/wasm32-unknown-unknown/release/shtts_web.wasm', '--target', target, '--out-dir', directory, '--out-name', 'shtts']);
  const wasm = `${directory}/shtts_bg.wasm`;
  run(optimizer, [wasm, '-o', wasm, '-O3', '--enable-bulk-memory', '--enable-nontrapping-float-to-int', '--strip-debug', '--strip-producers']);
}
