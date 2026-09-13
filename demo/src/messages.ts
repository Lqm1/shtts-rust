export type VoiceName = "Female" | "Male" | "Girl" | "Boy" | "Space";

export interface SynthesisRequest {
  kind: "synthesize";
  text: string;
  voice: VoiceName;
}

export type SynthesisResponse =
  | { kind: "ready" }
  | { kind: "audio"; wav: ArrayBuffer }
  | { kind: "error"; message: string };

export function parseVoice(value: string): VoiceName {
  switch (value) {
    case "Female":
    case "Male":
    case "Girl":
    case "Boy":
    case "Space":
      return value;
    default:
      throw new Error(`Unsupported voice: ${value}`);
  }
}
