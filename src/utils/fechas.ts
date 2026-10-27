/**
 * Convierte una fecha guardada en la base a `Date`.
 *
 * SQLite guarda los instantes en UTC con el formato de `CURRENT_TIMESTAMP`
 * (`YYYY-MM-DD HH:MM:SS`), sin zona. `new Date()` lee ese texto como hora local, y
 * en Buenos Aires corre todo 3 horas. Acá se marca como UTC; un texto que ya trae
 * zona (ISO con `Z` u offset) se respeta tal cual.
 */
export function fechaDeLaBase(texto: string): Date {
  const sinZona = /^\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}(:\d{2}(\.\d+)?)?$/;
  if (sinZona.test(texto)) {
    return new Date(`${texto.replace(' ', 'T')}Z`);
  }
  return new Date(texto);
}
