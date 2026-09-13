const HEADER_SIZE = 44;
const BYTES_PER_SAMPLE = 2;

/** Encode mono signed 16-bit PCM as a little-endian WAV file. */
export function encodeWav(pcm: Int16Array, sampleRate: number): ArrayBuffer {
  const buffer = new ArrayBuffer(HEADER_SIZE + pcm.byteLength);
  const view = new DataView(buffer);

  function writeText(offset: number, value: string): void {
    for (let i = 0; i < value.length; i++) {
      view.setUint8(offset + i, value.charCodeAt(i));
    }
  }

  writeText(0, "RIFF");
  view.setUint32(4, buffer.byteLength - 8, true);
  writeText(8, "WAVE");
  writeText(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true); // PCM format
  view.setUint16(22, 1, true); // Mono
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * BYTES_PER_SAMPLE, true);
  view.setUint16(32, BYTES_PER_SAMPLE, true);
  view.setUint16(34, 16, true);
  writeText(36, "data");
  view.setUint32(40, pcm.byteLength, true);

  pcm.forEach((sample, index) => {
    view.setInt16(HEADER_SIZE + index * BYTES_PER_SAMPLE, sample, true);
  });

  return buffer;
}
