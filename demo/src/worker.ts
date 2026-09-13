import init, { synthesize, Settings, Voice, sampleRate } from "shtts-wasm";
import wasmUrl from "shtts-wasm/shtts_bg.wasm?url";
import type { SynthesisRequest, SynthesisResponse } from "./messages";
import { encodeWav } from "./wav";

await init({ module_or_path: wasmUrl });
postMessage({ kind: "ready" } satisfies SynthesisResponse);

self.addEventListener("message", ({ data }: MessageEvent<SynthesisRequest>) => {
  const settings = Settings.preset(Voice[data.voice]);
  try {
    const kana = data.text
      .slice(0, 500)
      .normalize("NFKC")
      .replace(/[\u3041-\u3096]/g, (char) => String.fromCharCode(char.charCodeAt(0) + 0x60));
    const pcm = synthesize(kana, settings);
    if (!pcm.length) {
      throw new Error("No speech was generated. Enter Japanese kana and try again.");
    }

    const wav = encodeWav(pcm, sampleRate());
    postMessage({ kind: "audio", wav } satisfies SynthesisResponse, [wav]);
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : "Speech synthesis failed.";
    postMessage({ kind: "error", message } satisfies SynthesisResponse);
  } finally {
    settings.free();
  }
});
