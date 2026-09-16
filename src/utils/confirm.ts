import type { AppConfig } from '../types/config';

/**
 * Reversibilidad de una operación de borrado.
 *
 * - `reversible`: lo borrado se puede recuperar (papelera, o un registro que el
 *   usuario puede volver a crear sin perder nada valioso).
 * - `irreversible`: no hay vuelta atrás (purgar de la papelera, vaciarla).
 */
export type DeleteReversibility = 'reversible' | 'irreversible';

/**
 * ¿Hay que pedirle confirmación al usuario antes de este borrado?
 *
 * `ui.confirm_delete` gobierna SOLO las operaciones reversibles. Las
 * irreversibles confirman siempre, sin consultar el flag: apagar una preferencia
 * de comodidad no puede habilitar la pérdida definitiva de datos con un click.
 *
 * El default es confirmar. Si no hay config cargada (todavía no resolvió
 * `get_config`, o el componente se montó sin ConfigProvider), confirmamos igual:
 * ante la duda, en un camino destructivo se pregunta.
 */
export function shouldConfirm(
  kind: DeleteReversibility,
  config: Pick<AppConfig, 'ui'> | null | undefined
): boolean {
  if (kind === 'irreversible') return true;
  return config?.ui?.confirm_delete ?? true;
}
