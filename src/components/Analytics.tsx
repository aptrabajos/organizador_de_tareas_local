import { Component, createSignal, onMount, Show, For } from 'solid-js';
import { getProjectStats } from '../services/api';
import type { ProjectStats } from '../types/project';

const Analytics: Component = () => {
  const [stats, setStats] = createSignal<ProjectStats | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  const loadStats = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await getProjectStats();
      setStats(data);
    } catch (err) {
      console.error('Error al cargar estadísticas:', err);
      setError('Error al cargar estadísticas');
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    loadStats();
  });

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return new Intl.DateTimeFormat('es-ES', {
      day: '2-digit',
      month: 'short',
      hour: '2-digit',
      minute: '2-digit',
    }).format(date);
  };

  const getActivityIcon = (activityType: string) => {
    switch (activityType) {
      case 'opened':
        return '🚀';
      case 'edited':
        return '✏️';
      case 'backup':
        return '💾';
      default:
        return '📌';
    }
  };

  return (
    <div class="space-y-6">
      {/* Header */}
      <div class="flex items-center justify-between">
        <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
          📊 Estadísticas y Analytics
        </h2>
        <button
          onClick={loadStats}
          class="rounded bg-blue-600 px-3 py-2 text-sm text-white hover:bg-blue-700"
          disabled={loading()}
        >
          🔄 Actualizar
        </button>
      </div>

      {/* Loading State */}
      <Show when={loading()}>
        <div class="text-center text-gray-600 dark:text-gray-400">
          Cargando estadísticas...
        </div>
      </Show>

      {/* Error State */}
      <Show when={error()}>
        <div class="rounded-lg bg-red-50 p-4 text-red-800 dark:bg-red-900/20 dark:text-red-200">
          {error()}
        </div>
      </Show>

      {/* Stats Cards */}
      <Show when={!loading() && stats()}>
        <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          {/* Total Projects */}
          <div class="rounded-lg border border-gray-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-800">
            <div class="text-sm text-gray-600 dark:text-gray-400">
              Total de Proyectos
            </div>
            <div class="mt-2 text-3xl font-bold text-gray-900 dark:text-white">
              {stats()?.total_projects || 0}
            </div>
          </div>

          {/* Active Today */}
          <div class="rounded-lg border border-gray-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-800">
            <div class="text-sm text-gray-600 dark:text-gray-400">
              Activos Hoy
            </div>
            <div class="mt-2 text-3xl font-bold text-green-600 dark:text-green-400">
              {stats()?.active_today || 0}
            </div>
          </div>

          {/* Total Time */}
          <div class="rounded-lg border border-gray-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-800">
            <div class="text-sm text-gray-600 dark:text-gray-400">
              Tiempo Total (horas)
            </div>
            <div class="mt-2 text-3xl font-bold text-purple-600 dark:text-purple-400">
              {stats()?.total_time_hours.toFixed(1) || '0.0'}
            </div>
          </div>

          {/* Most Active Project */}
          <div class="rounded-lg border border-gray-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-800">
            <div class="text-sm text-gray-600 dark:text-gray-400">
              Más Activo
            </div>
            <div class="mt-2 truncate text-xl font-bold text-blue-600 dark:text-blue-400">
              {stats()?.most_active_project || 'N/A'}
            </div>
          </div>
        </div>

        {/* Activity Timeline */}
        <div class="rounded-lg border border-gray-200 bg-white p-6 shadow-sm dark:border-gray-700 dark:bg-gray-800">
          <h3 class="mb-4 text-xl font-semibold text-gray-900 dark:text-white">
            📅 Actividad Reciente
          </h3>

          <Show
            when={stats()?.recent_activities && stats()!.recent_activities.length > 0}
            fallback={
              <p class="text-center text-gray-500 dark:text-gray-400">
                No hay actividad registrada
              </p>
            }
          >
            <div class="space-y-3">
              <For each={stats()?.recent_activities}>
                {(activity) => (
                  <div class="flex items-start gap-3 rounded-lg border border-gray-100 p-3 transition-colors hover:bg-gray-50 dark:border-gray-700 dark:hover:bg-gray-700/50">
                    <div class="text-2xl">
                      {getActivityIcon(activity.activity_type)}
                    </div>
                    <div class="flex-1">
                      <div class="flex items-center justify-between">
                        <span class="font-medium text-gray-900 dark:text-white">
                          {activity.description || activity.activity_type}
                        </span>
                        <span class="text-xs text-gray-500 dark:text-gray-400">
                          {formatDate(activity.created_at)}
                        </span>
                      </div>
                      <Show when={activity.duration_seconds}>
                        <div class="mt-1 text-sm text-gray-600 dark:text-gray-400">
                          ⏱️ {Math.floor(activity.duration_seconds! / 60)} min
                        </div>
                      </Show>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default Analytics;
