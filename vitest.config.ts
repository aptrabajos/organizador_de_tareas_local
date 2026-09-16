import { defineConfig } from 'vitest/config';
import solid from 'vite-plugin-solid';

export default defineConfig({
  plugins: [solid({ ssr: false })],
  resolve: {
    conditions: ['development', 'browser'],
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test-setup.ts'],
    // Sin un `include` explícito, vitest aplica su patrón por defecto a TODO el
    // repositorio: así venía ejecutando la copia duplicada del frontend que vivía
    // en win10/source/src/ (hoy archivada fuera del índice, en docs/historico/).
    // Tests que corren contra código histórico dan falsa confianza, que es peor
    // que no tenerlos. La única fuente de tests del frontend es src/.
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    exclude: [
      'e2e/**',
      'node_modules/**',
      'dist/**',
      'docs/**',
      '_archivo_local/**',
    ],
  },
});
