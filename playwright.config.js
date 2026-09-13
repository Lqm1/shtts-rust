import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/browser',
  use: { baseURL: 'http://127.0.0.1:4173/shtts-rust/' },
  webServer: { command: 'npx vite preview demo --host 127.0.0.1 --port 4173', url: 'http://127.0.0.1:4173/shtts-rust/', reuseExistingServer: false },
});
