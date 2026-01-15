import { Component, createResource, For, Show } from 'solid-js';
import { getDashboardData } from '../services/api';
import type { DashboardData } from '../types/dashboard';

const Dashboard: Component = () => {
  console.log('Dashboard component mounted');
  const [data] = createResource<DashboardData>(() => {
    console.log('createResource fetching data...');
    return getDashboardData();
  });

  // Log para ver el estado del recurso
  data.state === 'ready' && console.log('Dashboard data loaded:', data());
  data.state === 'errored' &&
    console.error('Dashboard data error:', data.error);

  // Formateador de fecha simple
  const formatDate = (dateString: string) => {
    try {
      return new Date(dateString).toLocaleDateString('es-ES', {
        day: 'numeric',
        month: 'short',
        year: 'numeric',
      });
    } catch (e) {
      return dateString;
    }
  };

  return (
    <div class="min-h-screen bg-gray-50 p-4 dark:bg-gray-900">
      <h1 class="mb-6 text-3xl font-bold text-gray-800 dark:text-white">
        Dashboard
      </h1>

      <Show when={data.loading}>
        <div class="text-center text-gray-500 dark:text-gray-400">
          Cargando datos del dashboard...
        </div>
      </Show>

      <Show when={data.error}>
        <div
          class="relative rounded border border-red-400 bg-red-100 px-4 py-3 text-red-700"
          role="alert"
        >
          <strong class="font-bold">Error:</strong>
          <span class="block sm:inline">
            {' '}
            {data.error.message || 'Error desconocido'}
          </span>
        </div>
      </Show>

      <Show when={data() && !data.loading && !data.error}>
        <div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
          {/* Columna: Proyectos Recientes */}
          <div class="rounded-lg bg-white p-6 shadow dark:bg-gray-800">
            <h2 class="mb-4 text-xl font-semibold text-gray-700 dark:text-white">
              🚀 Proyectos Recientes
            </h2>
            <ul class="space-y-3">
              <For
                each={data()!.recent_projects}
                fallback={
                  <li class="text-gray-500 dark:text-gray-400">
                    No hay proyectos recientes.
                  </li>
                }
              >
                {(project) => (
                  <li class="rounded-md bg-gray-100 p-3 transition hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600">
                    <p class="font-semibold text-blue-600 dark:text-blue-400">
                      {project.name}
                    </p>
                    <p class="text-xs text-gray-500 dark:text-gray-400">
                      Abierto por última vez:{' '}
                      {formatDate(project.last_opened_at!)}
                    </p>
                  </li>
                )}
              </For>
            </ul>
          </div>

          {/* Columna: Tareas Pendientes */}
          <div class="rounded-lg bg-white p-6 shadow dark:bg-gray-800">
            <h2 class="mb-4 text-xl font-semibold text-gray-700 dark:text-white">
              ✅ Tareas Pendientes
            </h2>
            <ul class="max-h-96 space-y-3 overflow-y-auto">
              <For
                each={data()!.pending_todos}
                fallback={
                  <li class="text-gray-500 dark:text-gray-400">
                    ¡Ninguna tarea pendiente!
                  </li>
                }
              >
                {(item) => (
                  <li class="rounded-md bg-yellow-50 p-3 dark:bg-yellow-900/50">
                    <p class="font-medium text-gray-800 dark:text-gray-200">
                      {item.content}
                    </p>
                    <p class="text-xs text-gray-500 dark:text-gray-400">
                      Proyecto:{' '}
                      <span class="font-semibold">{item.project_name}</span>
                    </p>
                  </li>
                )}
              </For>
            </ul>
          </div>

          {/* Columna: Actividad Reciente */}
          <div class="rounded-lg bg-white p-6 shadow dark:bg-gray-800">
            <h2 class="mb-4 text-xl font-semibold text-gray-700 dark:text-white">
              📓 Actividad Reciente
            </h2>
            <ul class="space-y-3">
              <For
                each={data()!.recent_journal_entries}
                fallback={
                  <li class="text-gray-500 dark:text-gray-400">
                    No hay entradas de diario recientes.
                  </li>
                }
              >
                {(item) => (
                  <li class="rounded-r-md border-l-4 border-indigo-500 bg-gray-50 p-3 dark:bg-gray-700">
                    <p class="truncate text-sm text-gray-800 dark:text-gray-200">
                      {item.content}
                    </p>
                    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                      <span class="font-semibold">{item.project_name}</span> -{' '}
                      {formatDate(item.created_at)}
                    </p>
                  </li>
                )}
              </For>
            </ul>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default Dashboard;
