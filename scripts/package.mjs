import { readFile, writeFile, copyFile, mkdir } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';

const pkg = JSON.parse(await readFile('target/web/package.json', 'utf8'));
const cargo = await readFile('bindings/web/Cargo.toml', 'utf8');
const version = process.env.BOOTSTRAP_VERSION || cargo.match(/^version = "([^"]+)"/m)[1];
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('This workflow only publishes stable semantic versions');
if (process.env.GITHUB_REF_TYPE === 'tag' && process.env.GITHUB_REF_NAME !== `v${version}`) {
  throw new Error('Release tag must match bindings/web/Cargo.toml version');
}
Object.assign(pkg, {
  name: 'shtts', version,
  description: 'Deterministic speech synthesis in WebAssembly for browsers and JavaScript runtimes',
  repository: { type: 'git', url: 'git+https://github.com/Lqm1/shtts-rust.git' },
  homepage: 'https://Lqm1.github.io/shtts-rust/',
  license: 'UNLICENSED',
  exports: { '.': { types: './shtts.d.ts', import: './shtts.js' }, './shtts_bg.wasm': './shtts_bg.wasm', './package.json': './package.json' },
  publishConfig: { access: 'public', registry: 'https://registry.npmjs.org/' },
});
await writeFile('target/web/package.json', JSON.stringify(pkg, null, 2) + '\n');
await copyFile('README.md', 'target/web/README.md');
await mkdir('target/npm', { recursive: true });
const result = JSON.parse(execFileSync(process.execPath, [process.env.npm_execpath, 'pack', './target/web', '--pack-destination', 'target/npm', '--json', '--ignore-scripts'], { encoding: 'utf8' }))[0];
await writeFile('target/npm/manifest.json', JSON.stringify({ name: pkg.name, version, filename: result.filename, integrity: result.integrity }, null, 2));
console.log(`${pkg.name}@${version}: ${result.filename}`);
