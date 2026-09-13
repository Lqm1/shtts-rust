import './style.css';
import pkg from 'shtts-wasm/package.json';

document.querySelector('#version').textContent = `${pkg.name} v${pkg.version}`;
const worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
const button = document.querySelector('#generate');
const status = document.querySelector('#status');
const audio = document.querySelector('audio');
let audioUrl;
worker.onmessage = ({ data }) => {
  button.disabled = false;
  if (data.error) { status.textContent = data.error; return; }
  if (data.ready) { status.textContent = 'Ready to synthesize.'; return; }
  if (audioUrl) URL.revokeObjectURL(audioUrl);
  audioUrl = URL.createObjectURL(new Blob([data.wav], { type: 'audio/wav' }));
  audio.src = audioUrl;
  audio.hidden = false;
  status.textContent = 'Audio generated. Press play to listen.';
};
worker.onerror = () => { status.textContent = 'Could not load the speech engine. Reload the page to try again.'; button.disabled = true; };
document.querySelector('form').addEventListener('submit', event => {
  event.preventDefault();
  const text = document.querySelector('#text').value.trim();
  if (!text) { status.textContent = 'Enter some Japanese kana to synthesize.'; return; }
  button.disabled = true;
  audio.pause();
  status.textContent = 'Generating audio…';
  worker.postMessage({ text, voice: document.querySelector('#voice').value });
});
