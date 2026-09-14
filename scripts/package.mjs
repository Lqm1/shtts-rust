import { readFile, writeFile, copyFile, mkdir, readdir } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

process.chdir(fileURLToPath(new URL('../', import.meta.url)));

const targets = (await readdir('target/package', { withFileTypes: true }))
  .filter((entry) => entry.isDirectory()).map((entry) => entry.name).sort();
if (targets.length === 0) throw new Error('Run build:wasm before packing');
const pkg = { type: 'module', files: targets };
const cargo = await readFile('bindings/wasm/Cargo.toml', 'utf8');
const version = process.env.BOOTSTRAP_VERSION || cargo.match(/^version = "([^"]+)"/m)[1];
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('This workflow only publishes stable semantic versions');
if (process.env.GITHUB_REF_TYPE === 'tag' && process.env.GITHUB_REF_NAME !== `v${version}`) {
  throw new Error('Release tag must match bindings/wasm/Cargo.toml version');
}
Object.assign(pkg, {
  name: 'shtts-wasm', version,
  description: 'Deterministic speech synthesis in WebAssembly for browsers and JavaScript runtimes',
  repository: { type: 'git', url: 'git+https://github.com/Lqm1/shtts-rust.git' },
  homepage: 'https://Lqm1.github.io/shtts-rust/',
  license: 'Apache-2.0',
  exports: { './package.json': './package.json' },
  publishConfig: { access: 'public', registry: 'https://registry.npmjs.org/' },
});
for (const target of targets) {
  // Keep wasm-bindgen output intact. CommonJS needs its own package scope.
  await readFile(`target/package/${target}/shtts.js`);
  await readFile(`target/package/${target}/shtts.d.ts`);
  await readFile(`target/package/${target}/shtts_bg.wasm`);
  await writeFile(`target/package/${target}/package.json`, JSON.stringify({ type: target === 'nodejs' || target === 'no-modules' ? 'commonjs' : 'module' }) + '\n');
  pkg.exports[`./${target}`] = { types: `./${target}/shtts.d.ts`, default: `./${target}/shtts.js` };
  pkg.exports[`./${target}/*`] = `./${target}/*`;
}
await writeFile('target/package/package.json', JSON.stringify(pkg, null, 2) + '\n');
await copyFile('README.md', 'target/package/README.md');
await copyFile('LICENSE', 'target/package/LICENSE');
await mkdir('target/npm', { recursive: true });
const result = JSON.parse(execFileSync(process.execPath, [process.env.npm_execpath, 'pack', './target/package', '--pack-destination', 'target/npm', '--json', '--ignore-scripts'], { encoding: 'utf8' }))[0];
await writeFile('target/npm/manifest.json', JSON.stringify({ name: pkg.name, version, filename: result.filename, integrity: result.integrity }, null, 2));
console.log(`${pkg.name}@${version}: ${result.filename}`);
