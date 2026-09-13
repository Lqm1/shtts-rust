import "./style.css";
import pkg from "shtts-wasm/package.json";
import { parseVoice } from "./messages";
import type { SynthesisRequest, SynthesisResponse } from "./messages";

const form = document.querySelector("form");
const textInput = document.querySelector("#text");
const voiceInput = document.querySelector("#voice");
const generateButton = document.querySelector("#generate");
const status = document.querySelector("#status");
const audio = document.querySelector("audio");
const version = document.querySelector("#version");

if (
  !(form instanceof HTMLFormElement) ||
  !(textInput instanceof HTMLTextAreaElement) ||
  !(voiceInput instanceof HTMLSelectElement) ||
  !(generateButton instanceof HTMLButtonElement) ||
  !(status instanceof HTMLParagraphElement) ||
  !(audio instanceof HTMLAudioElement) ||
  !(version instanceof HTMLAnchorElement)
) {
  throw new Error("The demo page is missing a required element.");
}

version.textContent = `${pkg.name} v${pkg.version}`;
const worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
let audioUrl: string | undefined;

worker.addEventListener("message", ({ data }: MessageEvent<SynthesisResponse>) => {
  generateButton.disabled = false;

  switch (data.kind) {
    case "ready":
      status.textContent = "Ready to synthesize.";
      break;
    case "error":
      status.textContent = data.message;
      break;
    case "audio":
      if (audioUrl) URL.revokeObjectURL(audioUrl);
      audioUrl = URL.createObjectURL(new Blob([data.wav], { type: "audio/wav" }));
      audio.src = audioUrl;
      audio.hidden = false;
      status.textContent = "Audio generated. Press play to listen.";
      break;
    default: {
      const unexpected: never = data;
      throw new Error("Unexpected worker response", { cause: unexpected });
    }
  }
});

worker.addEventListener("error", () => {
  status.textContent = "The speech engine failed. Reload the page to try again.";
  generateButton.disabled = true;
});

form.addEventListener("submit", (event) => {
  event.preventDefault();
  const text = textInput.value.trim();
  if (!text) {
    status.textContent = "Enter some Japanese kana to synthesize.";
    return;
  }

  const request: SynthesisRequest = {
    kind: "synthesize",
    text,
    voice: parseVoice(voiceInput.value),
  };
  generateButton.disabled = true;
  audio.pause();
  status.textContent = "Generating audio…";
  worker.postMessage(request);
});

window.addEventListener("pagehide", (event) => {
  if (event.persisted) return;
  if (audioUrl) URL.revokeObjectURL(audioUrl);
  worker.terminate();
});
