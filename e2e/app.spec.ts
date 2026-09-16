import { test, expect, type Page } from '@playwright/test';

// Como la ventana de Tauri no es un navegador estándar, necesitamos una función
// para encontrar la página correcta que Playwright debe probar.
async function findTauriPage(page: Page): Promise<Page> {
  // La UI de la app corre en localhost:1420 (o el puerto configurado)
  // Esperamos a que la página cargue completamente
  await page.goto('/');
  await page.waitForLoadState('networkidle');
  return page;
}

test.describe('Smoke Test', () => {
  let appPage: Page;

  // Antes de cada test, encuentra la página de la app
  test.beforeEach(async ({ page }) => {
    appPage = await findTauriPage(page);
  });

  test('La aplicación se inicia y muestra el título principal', async () => {
    // Busca un encabezado (h1) que contenga "Gestor de Proyectos"
    const mainTitle = appPage.locator('h1', { hasText: 'Gestor de Proyectos' });

    // Comprueba que el título es visible en la página
    await expect(mainTitle).toBeVisible({ timeout: 15000 }); // Aumentar timeout por si el arranque es lento
  });

  test('El dashboard se muestra por defecto', async () => {
    // Busca un encabezado (h1) que contenga "Dashboard"
    const dashboardTitle = appPage.locator('h1', { hasText: 'Dashboard' });
    await expect(dashboardTitle).toBeVisible({ timeout: 10000 });
  });
});

// La CSP NO se puede verificar desde acá, y conviene que quede escrito para que nadie
// vuelva a intentarlo:
//
//   1. Esta suite corre Chromium (`devices['Desktop Chrome']`) contra el dev server de
//      Vite en localhost:1420. No abre el WebView de Tauri, que es quien recibe la
//      política.
//   2. Aunque lo abriera: en `tauri dev` sobre escritorio la CSP tampoco se emite. El
//      header sale del protocolo `tauri://`, que sirve los assets embebidos, y en
//      desktop dev el WebView navega directo al `devUrl` porque `PROXY_DEV_SERVER`
//      vale `cfg!(all(dev, mobile))`.
//
// Un caso de prueba acá pasaría SIEMPRE, incluso con una política que rompe la
// aplicación en release. Eso es peor que no tener gate.
//
// El gate real contra la regresión de configuración vive en
// `src-tauri/tests/security_config.rs` y corre con `cargo test`. La verificación de
// runtime exige un binario empaquetado manejado por WebDriver (`tauri build --debug`
// + `tauri-driver`), que es infraestructura que este repositorio todavía no tiene.
