import { useEffect, useRef, useState } from "react";
// Only import enum values here; the bundler entry initializes WASM in the Worker.
import { Voice, Emotion } from "shtts-wasm/web";
import pkg from "shtts-wasm/package.json";
import type { SynthesisRequest, SynthesisResponse } from "./worker";
import { parameters } from "./parameters";
import type { ParameterValues } from "./parameters";

const voices = Object.values(Voice).filter((value) => typeof value === "number");
const emotions = Object.values(Emotion).filter((value) => typeof value === "number");
const presetLabel = (name: string) => name.replace(/([a-z])([A-Z])/g, "$1 $2");

export default function App() {
  const [text, setText] = useState("");
  const [voice, setVoice] = useState(Voice.Female);
  const [emotion, setEmotion] = useState<Emotion | undefined>();
  const [values, setValues] = useState<ParameterValues | null>(null);
  const [busy, setBusy] = useState(true);
  const [status, setStatus] = useState("Loading the speech engine…");
  const [audioUrl, setAudioUrl] = useState("");
  const worker = useRef<Worker | null>(null);

  useEffect(() => {
    const engine = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
    worker.current = engine;
    engine.onmessage = ({ data }: MessageEvent<SynthesisResponse>) => {
      setBusy(false);
      if (data.kind === "preset") {
        setValues(data.values);
        setStatus("Preset loaded. Adjust the parameters or generate audio.");
      }
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
    if (!values) return;
    setBusy(true);
    setAudioUrl("");
    setStatus("Generating audio…");
    worker.current?.postMessage({ kind: "synthesize", text, values } satisfies SynthesisRequest);
  }

  function loadPreset(nextVoice: Voice, nextEmotion: Emotion | undefined) {
    setVoice(nextVoice);
    setEmotion(nextEmotion);
    setBusy(true);
    worker.current?.postMessage({
      kind: "preset",
      voice: nextVoice,
      emotion: nextEmotion,
    } satisfies SynthesisRequest);
  }

  return (
    <main>
      <header className="topbar">
        <span className="brand">
          SHTTS <small>VOICE STUDIO</small>
        </span>
        <a href="https://github.com/Lqm1/shtts-rust">GitHub ↗</a>
      </header>
      <div className="studio">
        <h1>Speech synthesis</h1>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            generate();
          }}
        >
          <section className="speech-panel" id="speech">
            <h2>
              <span>01</span> Text
            </h2>
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
          </section>
          <section className="preset-panel" id="presets">
            <h2>
              <span>02</span> Presets
            </h2>
            <fieldset disabled={busy} className="preset-controls">
              <div>
                <label htmlFor="voice">Voice</label>
                <select
                  id="voice"
                  value={voice}
                  onChange={(event) => loadPreset(Number(event.target.value), emotion)}
                >
                  {voices.map((value) => (
                    <option key={value} value={value}>
                      {presetLabel(Voice[value])}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label htmlFor="emotion">Emotion</label>
                <select
                  id="emotion"
                  value={emotion ?? ""}
                  onChange={(event) =>
                    loadPreset(
                      voice,
                      event.target.value === "" ? undefined : Number(event.target.value),
                    )
                  }
                >
                  <option value="">None</option>
                  {emotions.map((value) => (
                    <option key={value} value={value}>
                      {presetLabel(Emotion[value])}
                    </option>
                  ))}
                </select>
              </div>
              <button type="button" onClick={() => loadPreset(voice, emotion)}>
                Reset parameters
              </button>
            </fieldset>
            <p className="hint">
              Selecting a preset replaces all parameter values. You can fine-tune them below.
            </p>
          </section>
          <section className="tuning-panel" id="parameters">
            <h2>
              <span>03</span> Parameters
            </h2>
            <p className="section-caption">Adjust with sliders or enter exact values.</p>
            <fieldset
              disabled={busy || !values}
              className="parameters"
              aria-label="Voice parameters"
            >
              {values &&
                parameters.map(({ key, label, min, hint }) => (
                  <div className="parameter" key={key}>
                    <label htmlFor={key}>{label}</label>
                    <div className="parameter-inputs">
                      <input
                        type="range"
                        aria-label={`${label} slider`}
                        aria-describedby={`${key}-hint`}
                        min={min}
                        max={32767}
                        step={1}
                        value={values[key]}
                        onChange={(event) =>
                          setValues({ ...values, [key]: Number(event.target.value) })
                        }
                      />
                      <input
                        id={key}
                        type="number"
                        aria-describedby={`${key}-hint`}
                        min={min}
                        max={32767}
                        step={1}
                        required
                        value={values[key]}
                        onChange={(event) => {
                          const value = event.target.valueAsNumber;
                          if (Number.isInteger(value)) setValues({ ...values, [key]: value });
                        }}
                      />
                    </div>
                    <p id={`${key}-hint`} className="hint">
                      {hint}
                    </p>
                  </div>
                ))}
            </fieldset>
          </section>
          <div className="play-panel">
            <span>Audio is generated on your device.</span>
            <button disabled={busy || !text.trim() || !values}>▶ Generate audio</button>
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
      </div>
    </main>
  );
}
