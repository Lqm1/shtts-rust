import { synthesize, Settings, Voice, Emotion, sampleRate } from "shtts-wasm/bundler";
import { encodeWav } from "./wav";
import { parameters } from "./parameters";
import type { ParameterValues } from "./parameters";

export type SynthesisRequest =
  | { kind: "preset"; voice: Voice; emotion: Emotion | undefined }
  | { kind: "synthesize"; text: string; values: ParameterValues };
export type SynthesisResponse =
  | { kind: "preset"; values: ParameterValues }
  | { kind: "audio"; wav: ArrayBuffer }
  | { kind: "error"; message: string };

function loadPreset(voice: Voice, emotion?: Emotion) {
  const settings = Settings.preset(voice, emotion);
  try {
    const values: ParameterValues = {
      pitch: settings.pitch,
      accent: settings.accent,
      phraseAccent: settings.phraseAccent,
      volume: settings.volume,
      speed: settings.speed,
      spectral: settings.spectral,
      fluctuationDepth: settings.fluctuationDepth,
      fluctuationDelay: settings.fluctuationDelay,
      echoDepth: settings.echoDepth,
      echoDelay: settings.echoDelay,
      ringRate: settings.ringRate,
    };
    postMessage({ kind: "preset", values } satisfies SynthesisResponse);
  } finally {
    settings.free();
  }
}

loadPreset(Voice.Female);

self.addEventListener("message", ({ data }: MessageEvent<SynthesisRequest>) => {
  if (data.kind === "preset") {
    loadPreset(data.voice, data.emotion);
    return;
  }
  const settings = new Settings();
  try {
    for (const { key } of parameters) settings[key] = data.values[key];
    const kana = data.text
      .slice(0, 500)
      .normalize("NFKC")
      .replace(/[\u3041-\u3096]/g, (char) => String.fromCharCode(char.charCodeAt(0) + 0x60));
    const pcm = synthesize(kana, settings);
    if (!pcm.length) {
      throw new Error("No speech was generated. Enter Japanese kana and try again.");
    }

    const wav = encodeWav(pcm, sampleRate());
    postMessage({ kind: "audio", wav } satisfies SynthesisResponse, { transfer: [wav] });
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : "Speech synthesis failed.";
    postMessage({ kind: "error", message } satisfies SynthesisResponse);
  } finally {
    settings.free();
  }
});
