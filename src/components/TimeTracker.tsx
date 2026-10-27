import { Component, createSignal, createEffect, Show } from 'solid-js';
import type { TimeStats } from '../types/project';
import { getTimeStats } from '../services/api';
import { logger } from '../utils/logger';

interface TimeTrackerProps {
  projectId: number;
  projectPath: string;
  compact?: boolean;
}

// Formatear tiempo para display (ej: "2h 30m")
function formatTimeHuman(seconds: number): string {
  const hrs = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);

  if (hrs > 0 && mins > 0) {
    return `${hrs}h ${mins}m`;
  } else if (hrs > 0) {
    return `${hrs}h`;
  } else if (mins > 0) {
    return `${mins}m`;
  }
  return '< 1m';
}

// Componente para vista compacta
const CompactView: Component<{
  stats: TimeStats | null;
  isLoading: boolean;
}> = (props) => {
  return (
    <Show when={!props.isLoading}>
      <Show when={props.stats}>
        <div
          class="flex items-center space-x-1 rounded bg-blue-100 px-2 py-1 text-xs dark:bg-blue-900/30"
          title={`Total: ${formatTimeHuman(props.stats!.total_seconds)}, Hoy: ${formatTimeHuman(props.stats!.today_seconds)}`}
        >
          <svg
            class="h-3 w-3 text-blue-600 dark:text-blue-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <span class="font-medium text-blue-700 dark:text-blue-300">
            {formatTimeHuman(props.stats!.total_seconds)}
          </span>
        </div>
      </Show>
    </Show>
  );
};

// Componente para vista completa
const FullView: Component<{
  stats: TimeStats | null;
  isLoading: boolean;
}> = (props) => {
  return (
    <div class="rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800">
      <div class="mb-3 flex items-center justify-between">
        <h3 class="flex items-center space-x-2 text-sm font-medium text-gray-900 dark:text-white">
          <svg
            class="h-4 w-4 text-blue-500"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <span>Time Tracking</span>
        </h3>
      </div>

      <Show
        when={!props.isLoading}
        fallback={
          <div class="text-center text-sm text-gray-500">Cargando...</div>
        }
      >
        <Show when={props.stats}>
          <div class="grid grid-cols-2 gap-3">
            <div class="rounded-lg bg-gray-50 p-3 dark:bg-gray-700/50">
              <div class="text-xs text-gray-500 dark:text-gray-400">Total</div>
              <div class="text-lg font-semibold text-gray-900 dark:text-white">
                {formatTimeHuman(props.stats!.total_seconds)}
              </div>
            </div>
            <div class="rounded-lg bg-gray-50 p-3 dark:bg-gray-700/50">
              <div class="text-xs text-gray-500 dark:text-gray-400">Hoy</div>
              <div class="text-lg font-semibold text-blue-600 dark:text-blue-400">
                {formatTimeHuman(props.stats!.today_seconds)}
              </div>
            </div>
            <div class="rounded-lg bg-gray-50 p-3 dark:bg-gray-700/50">
              <div class="text-xs text-gray-500 dark:text-gray-400">
                Esta semana
              </div>
              <div class="text-lg font-semibold text-gray-900 dark:text-white">
                {formatTimeHuman(props.stats!.week_seconds)}
              </div>
            </div>
            <div class="rounded-lg bg-gray-50 p-3 dark:bg-gray-700/50">
              <div class="text-xs text-gray-500 dark:text-gray-400">
                Sesiones
              </div>
              <div class="text-lg font-semibold text-gray-900 dark:text-white">
                {props.stats!.session_count}
              </div>
            </div>
          </div>

          <Show when={props.stats!.avg_session_seconds > 0}>
            <div class="mt-3 flex items-center justify-between text-xs text-gray-500 dark:text-gray-400">
              <span>Promedio por sesion:</span>
              <span class="font-medium">
                {formatTimeHuman(props.stats!.avg_session_seconds)}
              </span>
            </div>
          </Show>

          <div class="mt-4 rounded-md bg-blue-50 p-3 dark:bg-blue-900/20">
            <div class="flex items-start space-x-2">
              <svg
                class="mt-0.5 h-4 w-4 flex-shrink-0 text-blue-500"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <div class="text-xs text-blue-700 dark:text-blue-300">
                <p class="font-medium">Tracking activo</p>
                <p class="mt-1 opacity-80">
                  El tiempo se registra al abrir la terminal del proyecto con el
                  boton Trabajar. La sesion se cierra sola al abrir otra o al
                  cerrar la aplicacion.
                </p>
              </div>
            </div>
          </div>
        </Show>
      </Show>
    </div>
  );
};

// El tiempo no depende de `.gestor/` (B2, 3.9 del diseño del tracker): las
// estadísticas salen de la base, así que se piden siempre. El tracker no crea esa
// carpeta, y con la rama "no activado" lo que mide quedaba invisible.
const TimeTracker: Component<TimeTrackerProps> = (props) => {
  const [stats, setStats] = createSignal<TimeStats | null>(null);
  const [isLoading, setIsLoading] = createSignal(true);

  const loadTrackingData = async (projectId: number) => {
    try {
      setIsLoading(true);
      setStats(await getTimeStats(projectId));
    } catch (error) {
      logger.error('Error loading tracking data:', error);
    } finally {
      setIsLoading(false);
    }
  };

  // Cargar datos cuando cambia el proyecto
  createEffect(() => {
    const id = props.projectId;
    if (id) {
      loadTrackingData(id);
    }
  });

  return (
    <Show
      when={props.compact}
      fallback={<FullView stats={stats()} isLoading={isLoading()} />}
    >
      <CompactView stats={stats()} isLoading={isLoading()} />
    </Show>
  );
};

export default TimeTracker;
