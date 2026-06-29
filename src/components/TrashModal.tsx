import { Component, createSignal, onMount, For, Show } from 'solid-js';
import { confirm } from '@tauri-apps/plugin-dialog';
import { getErrorMessage } from '../utils/errors';
import { listTrash } from '../services/api';
import type { TrashItem } from '../types/project';
import type { ProjectStore } from '../stores/projectStore';

interface TrashModalProps {
  store: ProjectStore;
  onClose: () => void;
}

const TrashModal: Component<TrashModalProps> = (props) => {
  const [items, setItems] = createSignal<TrashItem[]>([]);
  const [isLoading, setIsLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  // Recargar la lista de la papelera (se llama tras cada operación)
  async function reload() {
    setIsLoading(true);
    setError(null);
    try {
      setItems(await listTrash());
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setIsLoading(false);
    }
  }

  onMount(reload);

  // Restaurar: acción directa, sin confirmación.
  async function handleRestore(project: TrashItem) {
    try {
      await props.store.restoreProject(project.id);
      await reload();
    } catch (err) {
      setError(getErrorMessage(err));
    }
  }

  // Eliminar definitivamente: requiere confirmación explícita (irreversible).
  async function handlePurge(project: TrashItem) {
    const confirmed = await confirm(
      `Esto borra «${project.name}» y todos sus datos PARA SIEMPRE. No se puede deshacer.`,
      { title: 'Eliminar definitivamente', kind: 'warning' }
    );
    if (!confirmed) return;
    try {
      await props.store.purgeProject(project.id);
      await reload();
    } catch (err) {
      setError(getErrorMessage(err));
    }
  }

  // Vaciar papelera: confirma mostrando el conteo de proyectos afectados.
  async function handleEmpty() {
    const count = items().length;
    const confirmed = await confirm(
      `Esto borra ${count} proyecto(s) de la papelera y todos sus datos PARA SIEMPRE. No se puede deshacer.`,
      { title: 'Vaciar papelera', kind: 'warning' }
    );
    if (!confirmed) return;
    try {
      await props.store.emptyTrash();
      await reload();
    } catch (err) {
      setError(getErrorMessage(err));
    }
  }

  // Fecha de BORRADO real (deleted_at viaja en TrashItem). "Borrado el ..."
  function formatDate(item: TrashItem): string {
    return item.deleted_at ? `Borrado el ${item.deleted_at}` : '';
  }

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 p-4 dark:bg-opacity-70">
      <div class="max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-lg bg-white p-6 shadow-xl dark:bg-gray-800">
        {/* Header */}
        <div class="mb-6 flex items-center justify-between border-b border-gray-200 pb-4 dark:border-gray-700">
          <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
            🗑️ Papelera
          </h2>
          <button
            onClick={() => props.onClose()}
            class="rounded-lg p-2 text-gray-500 hover:bg-gray-100 hover:text-gray-700 dark:hover:bg-gray-700 dark:hover:text-gray-300"
          >
            ✕
          </button>
        </div>

        {/* Content */}
        <div class="space-y-4">
          <Show when={error()}>
            <p class="rounded-lg bg-red-50 p-3 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-400">
              {error()}
            </p>
          </Show>

          <Show
            when={!isLoading()}
            fallback={
              <p class="py-8 text-center text-sm text-gray-500 dark:text-gray-400">
                Cargando...
              </p>
            }
          >
            <Show
              when={items().length > 0}
              fallback={
                <p class="py-8 text-center text-sm text-gray-500 dark:text-gray-400">
                  La papelera está vacía.
                </p>
              }
            >
              <ul class="space-y-2">
                <For each={items()}>
                  {(project) => (
                    <li class="flex items-center justify-between gap-3 rounded-lg border border-gray-200 p-3 dark:border-gray-700">
                      <div class="min-w-0 flex-1">
                        <div class="flex items-center gap-2">
                          <span class="truncate font-medium text-gray-900 dark:text-white">
                            {project.name}
                          </span>
                          <Show when={(project.subproject_count ?? 0) > 0}>
                            <span class="rounded-full bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700 dark:bg-blue-900/30 dark:text-blue-400">
                              {project.subproject_count} subproyecto(s)
                            </span>
                          </Show>
                        </div>
                        <p class="truncate text-xs text-gray-500 dark:text-gray-400">
                          {formatDate(project)}
                        </p>
                      </div>
                      <div class="flex shrink-0 items-center gap-2">
                        <button
                          onClick={() => handleRestore(project)}
                          class="rounded-lg bg-emerald-100 px-3 py-1.5 text-sm font-medium text-emerald-700 hover:bg-emerald-200 dark:bg-emerald-900/30 dark:text-emerald-400 dark:hover:bg-emerald-900/50"
                        >
                          Restaurar
                        </button>
                        <button
                          onClick={() => handlePurge(project)}
                          class="rounded-lg bg-red-100 px-3 py-1.5 text-sm font-medium text-red-700 hover:bg-red-200 dark:bg-red-900/30 dark:text-red-400 dark:hover:bg-red-900/50"
                        >
                          Eliminar definitivamente
                        </button>
                      </div>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
          </Show>
        </div>

        {/* Footer */}
        <div class="mt-6 flex justify-end border-t border-gray-200 pt-4 dark:border-gray-700">
          <button
            onClick={handleEmpty}
            disabled={items().length === 0}
            class="rounded-lg bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-50"
          >
            Vaciar papelera
          </button>
        </div>
      </div>
    </div>
  );
};

export default TrashModal;
