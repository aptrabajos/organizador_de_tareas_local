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

// La CSP declarada en src-tauri/tauri.conf.json no se puede verificar con un test
// unitario: una política mal calibrada no rompe la compilación, rompe el render en
// silencio. Este caso convierte esa verificación manual en un gate automático.
test.describe('Content-Security-Policy', () => {
  test('la app carga sin emitir ninguna violación de CSP', async ({ page }) => {
    const violaciones: string[] = [];

    // Las violaciones llegan por dos canales distintos y hay que escuchar los dos:
    // el evento nativo securitypolicyviolation y el mensaje de consola del motor.
    page.on('console', (msg) => {
      const texto = msg.text();
      if (/Content[- ]Security[- ]Policy|Refused to (load|execute|apply|connect)/i.test(texto)) {
        violaciones.push(`[console:${msg.type()}] ${texto}`);
      }
    });

    await page.addInitScript(() => {
      document.addEventListener('securitypolicyviolation', (evento) => {
        // eslint-disable-next-line no-console
        console.error(
          `Content-Security-Policy violada: ${evento.violatedDirective} bloqueó ${evento.blockedURI}`
        );
      });
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Damos un margen para las cargas diferidas (fuentes, imágenes de proyecto).
    await page.waitForTimeout(2000);

    expect(
      violaciones,
      `La CSP bloqueó recursos que la app necesita:\n${violaciones.join('\n')}`
    ).toEqual([]);
  });
});
