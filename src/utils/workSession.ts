import toast from 'solid-toast';
import type { Project } from '../types/project';
import { trabajar, trackProjectOpen } from '../services/api';
import type { RespuestaTrabajar, SesionCortada } from '../services/api';
import { getErrorMessage } from './errors';
import { logger } from './logger';

/**
 * Duración legible para el aviso de "Parar" (ej: "1h 5m", "6m 57s", "3m", "40 s").
 * Por debajo de la hora muestra los segundos: truncar 6m 57s a "6m" escondía casi
 * un minuto de lo que se acababa de guardar.
 */
export function formatStoppedDuration(seconds: number): string {
  const hrs = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;
  if (hrs > 0) return mins > 0 ? `${hrs}h ${mins}m` : `${hrs}h`;
  if (mins > 0) return secs > 0 ? `${mins}m ${secs}s` : `${mins}m`;
  return `${seconds} s`;
}

/** Toast al empezar Trabajar cuando mide la app y se esperaba el tracker (U6). */
export const AVISO_SIN_TRACKER =
  'El tracker no está corriendo: esta sesión no se pausa por inactividad y se corta sola a las 12 h. Tocá Parar al terminar.';

/**
 * Flujo único de "Trabajar": registra la apertura y llama a `trabajar`, que crea o
 * reutiliza la sesión en la base y abre la terminal con su marca. Lo comparten la
 * tarjeta del proyecto y el Dashboard, para que los dos botones midan igual.
 *
 * La terminal la abre el backend (con la marca de la sesión): si `trabajar` falla,
 * el usuario lo ve en un toast en español (P7) y no se abre nada, porque una terminal
 * sin marca no mediría.
 *
 * Devuelve la sesión, o `null` si no se pudo empezar.
 */
export async function trabajarEnProyecto(
  project: Project
): Promise<RespuestaTrabajar | null> {
  // La analítica de aperturas no bloquea la medición: si falla, solo se registra.
  try {
    await trackProjectOpen(project.id);
  } catch (error) {
    logger.error('Error registrando la apertura del proyecto:', error);
  }

  try {
    const session = await trabajar(project.id);
    logger.debug(`🕒 Trabajar en ${session.project_name} (${session.dueno})`);

    if (session.tracking_initialized) {
      toast.success(`📁 Tracking activado para ${project.name}`, {
        duration: 2000,
      });
    }
    if (session.sesion_anterior) {
      toast('⏱️ Sesión anterior guardada', { duration: 1500, icon: '✓' });
    }
    if (session.aviso_sin_tracker) {
      toast(AVISO_SIN_TRACKER, { duration: 8000, icon: '⚠️' });
    }
    return session;
  } catch (error) {
    logger.error('Error al empezar a trabajar:', error);
    toast.error(
      `No se pudo empezar a trabajar en ${project.name}: ${getErrorMessage(error)}`,
      { duration: 6000 }
    );
    return null;
  }
}

/** Por qué terminó una sesión, en las palabras del desglose (3.10.15). */
const TEXTO_MOTIVO_SESION: Record<string, string> = {
  terminal_cerrada: 'cerraste la terminal',
  parar_app: 'tocaste Parar',
  parar_cli: 'corriste `gestor parar`',
  tracker_apagado: 'apagaste el tracker',
  tracker_caido:
    'el tracker se cortó; se contó hasta la última actividad registrada',
  proyecto_a_papelera: 'mandaste el proyecto a la papelera',
  restauracion: 'se restauró un backup',
  cambio_de_proyecto: 'empezaste a trabajar en otro proyecto',
  cierre_app: 'cerraste la app (sesión sin tracker)',
  huerfana: 'la app se cortó sin cerrar la sesión',
  tope_sin_tracker: 'llegó a 12 h sin el tracker y se cortó',
};

export function textoMotivoSesion(motivo: string): string {
  return TEXTO_MOTIVO_SESION[motivo] ?? motivo;
}

/**
 * Toast por cada sesión que se cortó al mandar su proyecto (o un ancestro) a la
 * papelera (3.10.9): un proyecto en la papelera no se mide.
 */
export function avisarSesionesCortadas(cortadas: SesionCortada[]): void {
  for (const cortada of cortadas) {
    toast(
      `Se cortó la sesión de ${cortada.nombre} (${formatStoppedDuration(cortada.segundos)}) porque lo mandaste a la papelera.`,
      { duration: 5000, icon: '🗑️' }
    );
  }
}
