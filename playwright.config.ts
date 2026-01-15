import { defineConfig, devices } from '@playwright/test';

// Lee las variables de entorno para una configuración flexible
const port = process.env.PORT || 1420;
const baseURL = `http://localhost:${port}`;

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',

  use: {
    baseURL,
    trace: 'on-first-retry',
  },

  // Iniciar el servidor de desarrollo de Tauri ANTES de que comiencen los tests
  webServer: {
    command: 'pnpm run tauri:dev',
    url: baseURL,
    reuseExistingServer: !process.env.CI,
    timeout: 120 * 1000, // 2 minutos para arrancar
    stdout: 'pipe',
    stderr: 'pipe',
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
