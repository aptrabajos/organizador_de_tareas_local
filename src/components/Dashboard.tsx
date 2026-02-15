import { Component, createResource, For, Show } from 'solid-js';
import {
  getDashboardData,
  openTerminal,
  trackProjectOpen,
} from '../services/api';
import type { DashboardData } from '../types/dashboard';
import type { Project } from '../types/project';
import toast from 'solid-toast';

interface DashboardProps {
  onProjectClick?: (project: Project) => void;
}

const Dashboard: Component<DashboardProps> = (props) => {
  console.log('Dashboard component mounted');
  const [data] = createResource<DashboardData>(() => {
    console.log('createResource fetching data...');
    return getDashboardData();
  });

  data.state === 'ready' && console.log('Dashboard data loaded:', data());
  data.state === 'errored' &&
    console.error('Dashboard data error:', data.error);

  const formatDate = (dateString: string) => {
    try {
      return new Date(dateString).toLocaleDateString('es-ES', {
        day: 'numeric',
        month: 'short',
        year: 'numeric',
      });
    } catch {
      return dateString;
    }
  };

  const formatRelativeTime = (dateString: string) => {
    try {
      const date = new Date(dateString);
      const now = new Date();
      const diffMs = now.getTime() - date.getTime();
      const diffMins = Math.floor(diffMs / 60000);
      const diffHours = Math.floor(diffMins / 60);
      const diffDays = Math.floor(diffHours / 24);

      if (diffMins < 1) return 'Justo ahora';
      if (diffMins < 60) return `Hace ${diffMins} min`;
      if (diffHours < 24) return `Hace ${diffHours}h`;
      if (diffDays < 7) return `Hace ${diffDays}d`;
      return formatDate(dateString);
    } catch {
      return dateString;
    }
  };

  const handleOpenProject = async (project: Project) => {
    try {
      // Registrar apertura para analytics
      await trackProjectOpen(project.id);
      // Abrir terminal en el proyecto
      await openTerminal(project.local_path);
      toast.success(`Abriendo ${project.name}...`, { duration: 2000 });
    } catch (error) {
      console.error('Error al abrir proyecto:', error);
      toast.error('Error al abrir el proyecto');
    }
  };

  const handleNavigateToProject = (project: Project) => {
    if (props.onProjectClick) {
      props.onProjectClick(project);
    }
  };

  return (
    <div class="min-h-[calc(100vh-140px)] p-4 sm:p-6">
      {/* Header */}
      <div class="mb-8">
        <h1 class="text-2xl font-bold text-surface-900 dark:text-white sm:text-3xl">
          Dashboard
        </h1>
        <p class="mt-1 text-sm text-surface-500 dark:text-surface-400">
          Resumen de tu actividad y proyectos recientes
        </p>
      </div>

      {/* Loading State */}
      <Show when={data.loading}>
        <div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
          <For each={[1, 2, 3]}>
            {() => (
              <div class="dashboard-card animate-pulse">
                <div class="mb-4 flex items-center gap-3">
                  <div class="h-10 w-10 rounded-xl bg-surface-200 dark:bg-surface-700" />
                  <div class="h-5 w-32 rounded bg-surface-200 dark:bg-surface-700" />
                </div>
                <div class="space-y-3">
                  <div class="h-16 rounded-lg bg-surface-200 dark:bg-surface-700" />
                  <div class="h-16 rounded-lg bg-surface-200 dark:bg-surface-700" />
                  <div class="h-16 rounded-lg bg-surface-200 dark:bg-surface-700" />
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>

      {/* Error State */}
      <Show when={data.error}>
        <div class="flex items-center gap-3 rounded-xl border border-rose-200 bg-rose-50 p-4 dark:border-rose-800 dark:bg-rose-900/20">
          <div class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full bg-rose-100 dark:bg-rose-900/50">
            <svg
              class="h-5 w-5 text-rose-600 dark:text-rose-400"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
          </div>
          <div>
            <p class="font-medium text-rose-800 dark:text-rose-200">
              Error al cargar el dashboard
            </p>
            <p class="text-sm text-rose-600 dark:text-rose-300">
              {data.error.message || 'Error desconocido'}
            </p>
          </div>
        </div>
      </Show>

      {/* Data Loaded */}
      <Show when={data() && !data.loading && !data.error}>
        <div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
          {/* ═══════════════════════════════════════════════════════════════════
              Proyectos Recientes
              ═══════════════════════════════════════════════════════════════════ */}
          <div class="dashboard-card">
            <div class="dashboard-card-header">
              <div class="dashboard-card-icon from-accent-500 to-accent-600">
                <svg
                  class="h-5 w-5 text-white"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M13 10V3L4 14h7v7l9-11h-7z"
                  />
                </svg>
              </div>
              <div>
                <h2 class="font-semibold text-surface-900 dark:text-white">
                  Proyectos Recientes
                </h2>
                <p class="text-xs text-surface-500 dark:text-surface-400">
                  Últimos proyectos abiertos
                </p>
              </div>
            </div>

            <ul class="space-y-2">
              <For
                each={data()!.recent_projects}
                fallback={
                  <li class="rounded-lg bg-surface-100 p-4 text-center text-sm text-surface-500 dark:bg-surface-800 dark:text-surface-400">
                    No hay proyectos recientes
                  </li>
                }
              >
                {(project) => (
                  <li class="group rounded-lg border border-transparent bg-surface-50 p-3 transition-all hover:border-accent-200 hover:bg-accent-50/50 dark:bg-surface-800/50 dark:hover:border-accent-800 dark:hover:bg-accent-900/20">
                    <div class="flex items-start justify-between gap-3">
                      <div class="min-w-0 flex-1">
                        <p class="truncate font-medium text-surface-900 group-hover:text-accent-600 dark:text-surface-100 dark:group-hover:text-accent-400">
                          {project.name}
                        </p>
                        <p
                          class="mt-0.5 truncate text-xs text-surface-500 dark:text-surface-400"
                          title={project.local_path}
                        >
                          {formatRelativeTime(project.last_opened_at!)}
                        </p>
                      </div>
                      <div class="flex flex-shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
                        <button
                          onClick={() => handleNavigateToProject(project)}
                          class="rounded-lg p-1.5 text-surface-500 transition-colors hover:bg-surface-200 hover:text-surface-700 dark:hover:bg-surface-700 dark:hover:text-surface-300"
                          title="Ver proyecto"
                        >
                          <svg
                            class="h-4 w-4"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                          >
                            <path
                              stroke-linecap="round"
                              stroke-linejoin="round"
                              stroke-width="2"
                              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                            />
                            <path
                              stroke-linecap="round"
                              stroke-linejoin="round"
                              stroke-width="2"
                              d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
                            />
                          </svg>
                        </button>
                        <button
                          onClick={() => handleOpenProject(project)}
                          class="rounded-lg bg-accent-500 p-1.5 text-white transition-colors hover:bg-accent-600"
                          title="Abrir terminal"
                        >
                          <svg
                            class="h-4 w-4"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                          >
                            <path
                              stroke-linecap="round"
                              stroke-linejoin="round"
                              stroke-width="2"
                              d="M13 10V3L4 14h7v7l9-11h-7z"
                            />
                          </svg>
                        </button>
                      </div>
                    </div>
                  </li>
                )}
              </For>
            </ul>
          </div>

          {/* ═══════════════════════════════════════════════════════════════════
              Tareas Pendientes
              ═══════════════════════════════════════════════════════════════════ */}
          <div class="dashboard-card">
            <div class="dashboard-card-header">
              <div class="dashboard-card-icon from-amber-500 to-amber-600">
                <svg
                  class="h-5 w-5 text-white"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"
                  />
                </svg>
              </div>
              <div>
                <h2 class="font-semibold text-surface-900 dark:text-white">
                  Tareas Pendientes
                </h2>
                <p class="text-xs text-surface-500 dark:text-surface-400">
                  {data()!.pending_todos.length} tareas por completar
                </p>
              </div>
            </div>

            <ul class="scrollbar-thin max-h-80 space-y-2 overflow-y-auto">
              <For
                each={data()!.pending_todos}
                fallback={
                  <li class="flex flex-col items-center justify-center rounded-lg bg-emerald-50 p-6 text-center dark:bg-emerald-900/20">
                    <svg
                      class="mb-2 h-8 w-8 text-emerald-500"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                    <p class="font-medium text-emerald-700 dark:text-emerald-400">
                      ¡Todo completado!
                    </p>
                    <p class="text-xs text-emerald-600 dark:text-emerald-500">
                      No hay tareas pendientes
                    </p>
                  </li>
                }
              >
                {(item) => (
                  <li class="group flex items-start gap-3 rounded-lg border border-amber-100 bg-amber-50/50 p-3 dark:border-amber-900/30 dark:bg-amber-900/10">
                    <div class="mt-0.5 flex h-5 w-5 flex-shrink-0 items-center justify-center rounded border-2 border-amber-300 bg-white dark:border-amber-600 dark:bg-surface-800">
                      <span class="h-2 w-2 rounded-full bg-amber-400 opacity-0 transition-opacity group-hover:opacity-100" />
                    </div>
                    <div class="min-w-0 flex-1">
                      <p class="text-sm text-surface-800 dark:text-surface-200">
                        {item.content}
                      </p>
                      <p class="mt-1 text-xs text-surface-500 dark:text-surface-400">
                        <span class="font-medium text-amber-600 dark:text-amber-400">
                          {item.project_name}
                        </span>
                      </p>
                    </div>
                  </li>
                )}
              </For>
            </ul>
          </div>

          {/* ═══════════════════════════════════════════════════════════════════
              Actividad Reciente (Journal)
              ═══════════════════════════════════════════════════════════════════ */}
          <div class="dashboard-card">
            <div class="dashboard-card-header">
              <div class="dashboard-card-icon from-emerald-500 to-emerald-600">
                <svg
                  class="h-5 w-5 text-white"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253"
                  />
                </svg>
              </div>
              <div>
                <h2 class="font-semibold text-surface-900 dark:text-white">
                  Actividad Reciente
                </h2>
                <p class="text-xs text-surface-500 dark:text-surface-400">
                  Últimas entradas del diario
                </p>
              </div>
            </div>

            <ul class="space-y-2">
              <For
                each={data()!.recent_journal_entries}
                fallback={
                  <li class="rounded-lg bg-surface-100 p-4 text-center text-sm text-surface-500 dark:bg-surface-800 dark:text-surface-400">
                    No hay entradas de diario recientes
                  </li>
                }
              >
                {(item) => (
                  <li class="group relative rounded-lg border-l-4 border-emerald-500 bg-surface-50 p-3 transition-all hover:bg-surface-100 dark:bg-surface-800/50 dark:hover:bg-surface-800">
                    <p class="line-clamp-2 text-sm text-surface-700 dark:text-surface-300">
                      {item.content}
                    </p>
                    <div class="mt-2 flex items-center justify-between text-xs">
                      <span class="font-medium text-emerald-600 dark:text-emerald-400">
                        {item.project_name}
                      </span>
                      <span class="text-surface-400 dark:text-surface-500">
                        {formatDate(item.created_at)}
                      </span>
                    </div>
                  </li>
                )}
              </For>
            </ul>
          </div>
        </div>

        {/* ═══════════════════════════════════════════════════════════════════════
            Quick Stats (Optional row)
            ═══════════════════════════════════════════════════════════════════════ */}
        <div class="mt-6 grid grid-cols-2 gap-4 sm:grid-cols-4">
          <div class="glass-card flex items-center gap-3 p-4">
            <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-accent-500/20 to-accent-600/20 text-accent-600 dark:from-accent-400/20 dark:to-accent-500/20 dark:text-accent-400">
              <svg
                class="h-6 w-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                />
              </svg>
            </div>
            <div>
              <p class="text-2xl font-bold text-surface-900 dark:text-white">
                {data()!.recent_projects.length}
              </p>
              <p class="text-xs text-surface-500 dark:text-surface-400">
                Proyectos activos
              </p>
            </div>
          </div>

          <div class="glass-card flex items-center gap-3 p-4">
            <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-amber-500/20 to-amber-600/20 text-amber-600 dark:from-amber-400/20 dark:to-amber-500/20 dark:text-amber-400">
              <svg
                class="h-6 w-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
                />
              </svg>
            </div>
            <div>
              <p class="text-2xl font-bold text-surface-900 dark:text-white">
                {data()!.pending_todos.length}
              </p>
              <p class="text-xs text-surface-500 dark:text-surface-400">
                Tareas pendientes
              </p>
            </div>
          </div>

          <div class="glass-card flex items-center gap-3 p-4">
            <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-emerald-500/20 to-emerald-600/20 text-emerald-600 dark:from-emerald-400/20 dark:to-emerald-500/20 dark:text-emerald-400">
              <svg
                class="h-6 w-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                />
              </svg>
            </div>
            <div>
              <p class="text-2xl font-bold text-surface-900 dark:text-white">
                {data()!.recent_journal_entries.length}
              </p>
              <p class="text-xs text-surface-500 dark:text-surface-400">
                Entradas recientes
              </p>
            </div>
          </div>

          <div class="glass-card flex items-center gap-3 p-4">
            <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-rose-500/20 to-rose-600/20 text-rose-600 dark:from-rose-400/20 dark:to-rose-500/20 dark:text-rose-400">
              <svg
                class="h-6 w-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            </div>
            <div>
              <p class="text-2xl font-bold text-surface-900 dark:text-white">
                {new Date().toLocaleDateString('es-ES', { weekday: 'short' })}
              </p>
              <p class="text-xs text-surface-500 dark:text-surface-400">
                {new Date().toLocaleDateString('es-ES', {
                  day: 'numeric',
                  month: 'short',
                })}
              </p>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default Dashboard;
