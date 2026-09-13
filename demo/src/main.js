import './style.css';
import pkg from 'shtts/package.json';

document.querySelector('#version').textContent = `shtts v${pkg.version}`;
const worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
const button = document.querySelector('#generate');
const status = document.querySelector('#status');
const audio = document.querySelector('audio');
let audioUrl;
worker.onmessage = ({ data }) => {
  button.disabled = false;
  if (data.error) { status.textContent = data.error; return; }
  if (data.ready) { status.textContent = '準備できました。'; return; }
  if (audioUrl) URL.revokeObjectURL(audioUrl);
  audioUrl = URL.createObjectURL(new Blob([data.wav], { type: 'audio/wav' }));
  audio.src = audioUrl;
  audio.hidden = false;
  status.textContent = '音声を生成しました。再生ボタンで聞けます。';
};
worker.onerror = () => { status.textContent = '音声エンジンを読み込めませんでした。ページを再読み込みしてください。'; button.disabled = true; };
document.querySelector('form').addEventListener('submit', event => {
  event.preventDefault();
  const text = document.querySelector('#text').value.trim();
  if (!text) { status.textContent = '読み上げる文章を入力してください。'; return; }
  button.disabled = true;
  audio.pause();
  status.textContent = '音声を生成しています…';
  worker.postMessage({ text, voice: document.querySelector('#voice').value });
});
