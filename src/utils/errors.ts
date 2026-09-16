/**
 * Extrae un mensaje legible de un error de cualquier forma.
 *
 * IMPORTANTE: en Tauri v2, un comando que devuelve `Err(String)` rechaza la
 * promesa de `invoke()` con un STRING plano (no un objeto `Error`). El patrón
 * `err instanceof Error ? err.message : '...'` descarta ese string y pierde el
 * mensaje accionable del backend (ej. validaciones en español). Esta función
 * contempla los tres casos.
 */
export function getErrorMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  // Única excepción permitida a la regla `no-restricted-syntax` que prohíbe
  // `instanceof Error`: este es EL lugar donde se centraliza ese chequeo.
  // eslint-disable-next-line no-restricted-syntax
  if (err instanceof Error) return err.message;
  return 'Error desconocido';
}
