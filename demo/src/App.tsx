import { useEffect, useRef, useState } from "react";
import { Voice } from "shtts-wasm";
import pkg from "shtts-wasm/package.json";
import type { SynthesisRequest, SynthesisResponse } from "./worker";

const voices = [Voice.Female, Voice.Male, Voice.Girl, Voice.Boy, Voice.Space];

export default function App() {
  const [text, setText] = useState("");
  const [voice, setVoice] = useState(Voice.Female);
  const [busy, setBusy] = useState(true);
  const [status, setStatus] = useState("Loading the speech engine…");
  const [audioUrl, setAudioUrl] = useState("");
  const worker = useRef<Worker | null>(null);

  useEffect(() => {
    const engine = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
    worker.current = engine;
    engine.onmessage = ({ data }: MessageEvent<SynthesisResponse>) => {
      setBusy(false);
      if (data.kind === "ready") setStatus("Ready to synthesize.");
      if (data.kind === "error") setStatus(data.message);
      if (data.kind === "audio") {
        setAudioUrl(URL.createObjectURL(new Blob([data.wav], { type: "audio/wav" })));
        setStatus("Audio generated. Press play to listen.");
      }
    };
    engine.onerror = () => {
      setBusy(true);
      setStatus("The speech engine failed. Reload the page to try again.");
    };
    return () => engine.terminate();
  }, []);

  useEffect(
    () => () => {
      if (audioUrl) URL.revokeObjectURL(audioUrl);
    },
    [audioUrl],
  );

  function generate() {
    setBusy(true);
    setAudioUrl("");
    setStatus("Generating audio…");
    worker.current?.postMessage({ text, voice } satisfies SynthesisRequest);
  }

  return (
    <main>
      <header>
        <span className="eyebrow">RUST → WEBASSEMBLY</span>
        <a href="https://github.com/Lqm1/shtts-rust">GitHub ↗</a>
      </header>
      <h1>Give text a voice.</h1>
      <p className="intro">
        Try SHTTS speech synthesis in your browser. Your text stays on your device.
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          generate();
        }}
      >
        <label htmlFor="text">Text to speak</label>
        <textarea
          id="text"
          maxLength={500}
          rows={4}
          placeholder="Enter Japanese hiragana or katakana"
          value={text}
          onChange={(event) => setText(event.target.value)}
        />
        <p className="hint">
          Japanese kana only. Hiragana is converted to katakana. Up to 500 characters.
        </p>
        <div className="controls">
          <div>
            <label htmlFor="voice">Voice</label>
            <select
              id="voice"
              value={voice}
              onChange={(event) => setVoice(Number(event.target.value))}
            >
              {voices.map((value) => (
                <option key={value} value={value}>
                  {Voice[value]}
                </option>
              ))}
            </select>
          </div>
          <button disabled={busy || !text.trim()}>Generate audio</button>
        </div>
      </form>
      <p id="status" role="status">
        {status}
      </p>
      {audioUrl && <audio controls src={audioUrl} aria-label="Generated speech" />}
      <footer>
        <a href="https://www.npmjs.com/package/shtts-wasm">
          {pkg.name} v{pkg.version}
        </a>
        <span>11,025 Hz · mono PCM</span>
      </footer>
    </main>
  );
}
