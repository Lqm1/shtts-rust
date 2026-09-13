import init, { synthesize, Settings, Voice, sampleRate } from 'shtts-wasm';
import wasmUrl from 'shtts-wasm/shtts_bg.wasm?url';

await init({ module_or_path: wasmUrl });
postMessage({ ready: true });
onmessage = ({ data }) => {
  let settings;
  try {
    settings = Settings.preset(Voice[data.voice]);
    const kana = data.text.slice(0, 500).normalize('NFKC').replace(/[ぁ-ゖ]/g, char => String.fromCharCode(char.charCodeAt(0) + 0x60));
    const pcm = synthesize(kana, settings);
    if (!pcm.length) throw new Error('No speech was generated. Enter Japanese kana and try again.');
    const wav = new ArrayBuffer(44 + pcm.byteLength);
    const view = new DataView(wav);
    const text = (offset, value) => [...value].forEach((char, i) => view.setUint8(offset + i, char.charCodeAt(0)));
    text(0, 'RIFF'); view.setUint32(4, 36 + pcm.byteLength, true); text(8, 'WAVE'); text(12, 'fmt ');
    view.setUint32(16, 16, true); view.setUint16(20, 1, true); view.setUint16(22, 1, true);
    view.setUint32(24, sampleRate(), true); view.setUint32(28, sampleRate() * 2, true);
    view.setUint16(32, 2, true); view.setUint16(34, 16, true); text(36, 'data'); view.setUint32(40, pcm.byteLength, true);
    pcm.forEach((sample, i) => view.setInt16(44 + i * 2, sample, true));
    postMessage({ wav }, [wav]);
  } catch (error) { postMessage({ error: error.message }); }
  finally { settings?.free(); }
};
