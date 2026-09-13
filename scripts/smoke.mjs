import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import init, { synthesize, Settings, Voice, sampleRate } from 'shtts';

await init({ module_or_path: await readFile(new URL(import.meta.resolve('shtts/shtts_bg.wasm'))) });
const settings = Settings.preset(Voice.Female);
try {
  const pcm = synthesize('コンニチハ。', settings);
  assert(pcm instanceof Int16Array);
  assert(pcm.length > 0);
  assert(pcm.some(value => value !== 0));
  assert.equal(sampleRate(), 11025);
  assert.deepEqual(synthesize('コンニチハ。', settings), pcm);
  assert.throws(() => { settings.speed = NaN; });
  console.log(`Synthesis verified: ${pcm.length} samples at ${sampleRate()} Hz`);
} finally { settings.free(); }
