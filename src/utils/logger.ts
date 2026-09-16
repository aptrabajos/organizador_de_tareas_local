/**
 * Logging del frontend detrás de una guarda de entorno.
 *
 * Había 104 llamadas a `console.*` repartidas en 19 archivos de producción y
 * CERO guardas: `rg "import.meta.env.DEV" src/` no devolvía un solo resultado.
 * Todo ese diagnóstico —con emojis de marcador, objetos de proyecto enteros y
 * datos de formularios— se emitía igual en el build que se le entrega al
 * usuario.
 *
 * ## Por qué `error` también se silencia en producción
 *
 * Es la decisión menos obvia del módulo, así que queda escrita. En el build de
 * producción de esta app las devtools están apagadas (ver B9: la config de
 * ventana ya no las fuerza), así que NADIE puede abrir esa consola: un
 * `console.error` en producción no lo lee ni el usuario ni nosotros. No se está
 * perdiendo observabilidad, se está sacando peso muerto.
 *
 * Si algún día hiciera falta diagnóstico real en producción, el camino es
 * mandarlo al backend —que desde el B23 sí tiene logging con niveles y archivo—
 * a través de un comando Tauri. Ese es un cambio de alcance propio, no algo que
 * corresponda colgarle a este helper.
 *
 * ## Qué elimina el build de producción y qué no
 *
 * `import.meta.env.DEV` no es una lectura en runtime: Vite lo reemplaza
 * textualmente por `false` al compilar producción, así que cada cuerpo queda
 * como `if (!false) return;` y el minificador lo vacía.
 *
 * Lo que NO desaparece es la LLAMADA. Medido sobre `dist/` después del build:
 * los `logger.debug("📡 [STORE] ...")` siguen ahí con su string, porque el
 * bundler no puede asumir por su cuenta que invocar una función importada no
 * tiene efectos. Se probó marcarlas con `esbuild.pure` en vite.config y el
 * bundle salió byte por byte idéntico, así que esa config se descartó en vez de
 * dejarla decorativa.
 *
 * O sea: en producción no se EMITE nada —que es el defecto que había que
 * arreglar— pero los textos de diagnóstico siguen presentes en el bundle. Es un
 * costo de unos pocos KB sobre 293. Si algún día molesta, la única solución que
 * el bundler entiende es la guarda en el sitio de llamada
 * (`import.meta.env.DEV && logger.debug(...)`), y son 104 sitios: no vale la
 * pena hoy.
 *
 * La guarda va DENTRO de cada función, y no capturada en una `const` al
 * importar, porque así el módulo es testeable: el stub de entorno de un test
 * corre después del import y con una `const` no cambiaría nada.
 */

/** Diagnóstico de desarrollo. Reemplaza a los `console.log` sueltos. */
export function debug(...args: unknown[]): void {
  if (!import.meta.env.DEV) return;
  console.log(...args);
}

/** Algo salió de lo esperado pero la operación sigue. */
export function warn(...args: unknown[]): void {
  if (!import.meta.env.DEV) return;
  console.warn(...args);
}

/** Una operación falló. Ver arriba por qué tampoco sale en producción. */
export function error(...args: unknown[]): void {
  if (!import.meta.env.DEV) return;
  console.error(...args);
}

export const logger = { debug, warn, error };
