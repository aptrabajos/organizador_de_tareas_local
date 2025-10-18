import { createSignal, onMount, Show, For } from 'solid-js';
import type { Project, JournalEntry, ProjectTodo, ProjectLink } from '../types/project';
import {
  getProject,
  getJournalEntries,
  getProjectTodos,
  getProjectLinks,
} from '../services/api';

interface ProjectContextProps {
  projectId: number;
  onClose: () => void;
}

export default function ProjectContext(props: ProjectContextProps) {
  const [project, setProject] = createSignal<Project | null>(null);
  const [journalEntries, setJournalEntries] = createSignal<JournalEntry[]>([]);
  const [todos, setTodos] = createSignal<ProjectTodo[]>([]);
  const [links, setLinks] = createSignal<ProjectLink[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  // Obtener solo TODOs pendientes
  const pendingTodos = () => todos().filter((t) => !t.is_completed);

  // Limitar entradas de diario a las 5 más recientes
  const recentJournalEntries = () => journalEntries().slice(0, 5);

  // Dividir tags en array
  const projectTags = () => {
    const proj = project();
    if (!proj?.tags) return [];
    return proj.tags.split(',').map((tag) => tag.trim());
  };

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);

      // Cargar datos en paralelo
      const [projData, journalData, todosData, linksData] = await Promise.all([
        getProject(props.projectId),
        getJournalEntries(props.projectId),
        getProjectTodos(props.projectId),
        getProjectLinks(props.projectId),
      ]);

      setProject(projData);
      setJournalEntries(journalData);
      setTodos(todosData);
      setLinks(linksData);
    } catch (err) {
      console.error('Error al cargar contexto del proyecto:', err);
      setError('Error al cargar el contexto del proyecto');
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    loadData();
  });

  const handleBackdropClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget) {
      props.onClose();
    }
  };

  return (
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50"
      onClick={handleBackdropClick}
      data-testid="modal-backdrop"
    >
      <div class="flex max-h-[90vh] w-full max-w-4xl flex-col rounded-lg bg-white shadow-xl dark:bg-gray-800">
        {/* Header */}
        <div class="flex items-center justify-between border-b border-gray-200 px-6 py-4 dark:border-gray-700">
          <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
            Contexto del Proyecto
          </h2>
          <button
            onClick={props.onClose}
            class="rounded-lg p-2 text-gray-400 hover:bg-gray-100 hover:text-gray-600 dark:hover:bg-gray-700 dark:hover:text-gray-300"
            aria-label="Cerrar"
          >
            <span class="text-2xl">×</span>
          </button>
        </div>

        {/* Content */}
        <div class="flex-1 overflow-y-auto p-6">
          <Show when={loading()}>
            <div class="flex items-center justify-center py-12" role="status">
              <div class="text-center text-gray-500 dark:text-gray-400">
                Cargando contexto del proyecto...
              </div>
            </div>
          </Show>

          <Show when={error()}>
            <div class="rounded-lg bg-red-50 p-4 text-red-800 dark:bg-red-900/20 dark:text-red-400">
              {error()}
            </div>
          </Show>

          <Show when={!loading() && !error() && project()}>
            <div class="space-y-6">
              {/* Información del Proyecto */}
              <section>
                <h3 class="mb-3 text-xl font-semibold text-gray-900 dark:text-white">
                  {project()?.name}
                </h3>
                <p class="mb-3 text-gray-700 dark:text-gray-300">
                  {project()?.description}
                </p>
                <div class="mb-3">
                  <span class="text-sm text-gray-500 dark:text-gray-400">Ruta: </span>
                  <code class="rounded bg-gray-100 px-2 py-1 text-sm text-gray-800 dark:bg-gray-700 dark:text-gray-200">
                    {project()?.local_path}
                  </code>
                </div>
                <Show when={projectTags().length > 0}>
                  <div class="flex flex-wrap gap-2">
                    <For each={projectTags()}>
                      {(tag) => (
                        <span class="rounded-full bg-blue-100 px-3 py-1 text-sm text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                          {tag}
                        </span>
                      )}
                    </For>
                  </div>
                </Show>
                <Show when={project()?.status}>
                  <div class="mt-3">
                    <span class="inline-flex items-center rounded-full bg-green-100 px-3 py-1 text-sm font-medium text-green-800 dark:bg-green-900 dark:text-green-200">
                      {project()?.status}
                    </span>
                  </div>
                </Show>
              </section>

              {/* Diario Reciente */}
              <section>
                <h4 class="mb-3 text-lg font-semibold text-gray-900 dark:text-white">
                  📓 Diario Reciente
                </h4>
                <Show
                  when={recentJournalEntries().length > 0}
                  fallback={
                    <p class="text-sm text-gray-500 dark:text-gray-400">
                      Sin entradas de diario recientes
                    </p>
                  }
                >
                  <div class="space-y-3">
                    <For each={recentJournalEntries()}>
                      {(entry) => (
                        <div class="rounded-lg border border-gray-200 bg-gray-50 p-3 dark:border-gray-700 dark:bg-gray-900">
                          <p class="text-sm text-gray-700 dark:text-gray-300">
                            {entry.content}
                          </p>
                          <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                            {new Date(entry.created_at).toLocaleString('es-ES')}
                          </p>
                        </div>
                      )}
                    </For>
                  </div>
                </Show>
              </section>

              {/* Tareas Pendientes */}
              <section>
                <h4 class="mb-3 text-lg font-semibold text-gray-900 dark:text-white">
                  ✅ Tareas Pendientes
                </h4>
                <Show
                  when={pendingTodos().length > 0}
                  fallback={
                    <p class="text-sm text-gray-500 dark:text-gray-400">
                      Sin tareas pendientes
                    </p>
                  }
                >
                  <ul class="space-y-2">
                    <For each={pendingTodos()}>
                      {(todo) => (
                        <li class="flex items-start gap-2">
                          <span class="mt-0.5 text-gray-400">•</span>
                          <span class="text-sm text-gray-700 dark:text-gray-300">
                            {todo.content}
                          </span>
                        </li>
                      )}
                    </For>
                  </ul>
                </Show>
              </section>

              {/* Enlaces */}
              <section>
                <h4 class="mb-3 text-lg font-semibold text-gray-900 dark:text-white">
                  🔗 Enlaces
                </h4>
                <Show
                  when={links().length > 0}
                  fallback={
                    <p class="text-sm text-gray-500 dark:text-gray-400">Sin enlaces</p>
                  }
                >
                  <div class="space-y-2">
                    <For each={links()}>
                      {(link) => (
                        <div class="rounded-lg border border-gray-200 bg-gray-50 p-3 dark:border-gray-700 dark:bg-gray-900">
                          <a
                            href={link.url}
                            target="_blank"
                            rel="noopener noreferrer"
                            class="text-sm font-medium text-blue-600 hover:underline dark:text-blue-400"
                          >
                            {link.description || link.url}
                          </a>
                          <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                            {link.link_type}
                          </p>
                        </div>
                      )}
                    </For>
                  </div>
                </Show>
              </section>
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
}
