export const parameters = [
  {
    key: "pitch",
    label: "Pitch",
    min: -32768,
    hint: "Native pitch code; higher values raise the pitch.",
  },
  {
    key: "accent",
    label: "Accent",
    min: -32768,
    hint: "Initial accent modulation; 100 is neutral.",
  },
  {
    key: "phraseAccent",
    label: "Phrase accent",
    min: -32768,
    hint: "Following accent modulation; 100 is neutral.",
  },
  {
    key: "volume",
    label: "Volume",
    min: -32768,
    hint: "Logarithmic amplitude offset; 0 is neutral.",
  },
  {
    key: "speed",
    label: "Duration / speed",
    min: 1,
    hint: "Duration percentage; higher values speak more slowly.",
  },
  {
    key: "spectral",
    label: "Spectral scale",
    min: 1,
    hint: "Spectral scale percentage; 100 is neutral.",
  },
  {
    key: "fluctuationDepth",
    label: "Fluctuation depth",
    min: -32768,
    hint: "Pitch variation depth; 0 disables variation.",
  },
  {
    key: "fluctuationDelay",
    label: "Fluctuation delay",
    min: -32768,
    hint: "Pitch variation delay in samples.",
  },
  {
    key: "echoDepth",
    label: "Echo depth",
    min: -32768,
    hint: "Echo mix percentage; 0 disables echo.",
  },
  { key: "echoDelay", label: "Echo delay", min: -32768, hint: "Echo delay in samples." },
  {
    key: "ringRate",
    label: "Ring modulation",
    min: -32768,
    hint: "Modulation rate in native units; 0 disables the effect.",
  },
] satisfies {
  key: keyof import("shtts-wasm/bundler").Settings;
  label: string;
  min: number;
  hint: string;
}[];

export type ParameterValues = Record<(typeof parameters)[number]["key"], number>;
