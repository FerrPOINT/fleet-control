import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: 'preview-e2e',
  fullyParallel: true,
  workers: 3,
  retries: 0,
  reporter: 'list',
  outputDir: 'test-results/pm-draft-preview',
  use: { baseURL: 'http://127.0.0.1:55498', trace: 'retain-on-failure' },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
  ],
  webServer: {
    command: 'node node_modules/vite/bin/vite.js preview --config vite.pm-draft-preview.config.ts',
    url: 'http://127.0.0.1:55498/pm-draft-preview.html',
    reuseExistingServer: false,
    timeout: 60000,
  },
})
