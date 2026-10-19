import toast from 'solid-toast';
import type { Project } from '../types/project';
import { startWorkSession, trackProjectOpen } from '../services/api';
import type { WorkSessionResponse } from '../services/api';
import { getErrorMessage } from './errors';
import { logger } from './logger';

/**
 * Flujo único de "Trabajar": registra la apertura, arranca la sesión de tiempo y
 * abre el terminal. Lo comparten la tarjeta del proyecto y el Dashboard, para que
 * los dos botones midan igual.
 *
 * Si la sesión no arranca, el usuario lo ve (toast) y el terminal se abre igual:
 * no poder medir no es motivo para no dejarlo trabajar. Antes el error se tragaba
 * en un `catch` y el tiempo dejaba de contar sin que nadie se enterara (P7).
 *
 * Devuelve la sesión iniciada, o `null` si no se pudo iniciar.
 */
export async function trabajarEnProyecto(
  project: Project,
  abrirTerminal: (project: Project) => unknown
): Promise<WorkSessionResponse | null> {
  // La analítica de aperturas no bloquea la medición: si falla, solo se registra.
  try {
    await trackProjectOpen(project.id);
  } catch (error) {
    logger.error('Error registrando la apertura del proyecto:', error);
  }

  let session: WorkSessionResponse | null = null;
  try {
    session = await startWorkSession(project.id);
    logger.debug(`🕒 Sesión de trabajo iniciada: ${session.project_name}`);

    if (session.tracking_initialized) {
      toast.success(`📁 Tracking activado para ${project.name}`, {
        duration: 2000,
      });
    }
    if (session.previous_session_stopped) {
      toast('⏱️ Sesión anterior guardada', { duration: 1500, icon: '✓' });
    }
  } catch (error) {
    logger.error('Error al iniciar la sesión de trabajo:', error);
    toast.error(
      `No se pudo empezar a medir el tiempo de ${project.name}: ${getErrorMessage(error)}. El terminal se abre igual.`,
      { duration: 6000 }
    );
  }

  await abrirTerminal(project);
  return session;
}
