import { createSignal, createEffect, For, Show } from 'solid-js';
import type { ProjectTodo } from '../types/project';
import {
  getProjectTodos,
  createTodo,
  updateTodo,
  deleteTodo,
} from '../services/api';

interface TodoListProps {
  projectId: number;
}

export default function TodoList(props: TodoListProps) {
  const [todos, setTodos] = createSignal<ProjectTodo[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [newTodoContent, setNewTodoContent] = createSignal('');

  // Cargar TODOs al montar
  createEffect(() => {
    loadTodos();
  });

  const loadTodos = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await getProjectTodos(props.projectId);
      setTodos(data);
    } catch (err) {
      console.error('Error loading todos:', err);
      setError('Error al cargar las tareas');
    } finally {
      setLoading(false);
    }
  };

  const handleCreate = async () => {
    const content = newTodoContent().trim();
    if (!content) return;

    try {
      await createTodo({
        project_id: props.projectId,
        content,
      });
      setNewTodoContent('');
      await loadTodos();
    } catch (err) {
      console.error('Error creating todo:', err);
      setError('Error al crear tarea');
    }
  };

  const handleToggle = async (todo: ProjectTodo) => {
    try {
      await updateTodo(todo.id, {
        is_completed: !todo.is_completed,
      });
      await loadTodos();
    } catch (err) {
      console.error('Error toggling todo:', err);
      setError('Error al actualizar tarea');
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm('¿Eliminar esta tarea?')) return;

    try {
      await deleteTodo(id);
      await loadTodos();
    } catch (err) {
      console.error('Error deleting todo:', err);
      setError('Error al eliminar tarea');
    }
  };

  const pendingTodos = () => todos().filter((t) => !t.is_completed);
  const completedTodos = () => todos().filter((t) => t.is_completed);

  return (
    <div class="flex flex-col gap-4">
      {/* Error Message */}
      <Show when={error()}>
        <div class="rounded bg-red-100 p-3 text-sm text-red-700 dark:bg-red-900 dark:text-red-200">
          {error()}
        </div>
      </Show>

      {/* Loading State */}
      <Show when={loading()}>
        <div role="status" class="py-4 text-center text-gray-500">
          Cargando tareas...
        </div>
      </Show>

      {/* Input para nuevo TODO */}
      <Show when={!loading()}>
        <div class="flex gap-2">
          <input
            type="text"
            value={newTodoContent()}
            onInput={(e) => setNewTodoContent(e.currentTarget.value)}
            onKeyPress={(e) => {
              if (e.key === 'Enter') {
                handleCreate();
              }
            }}
            placeholder="Nueva tarea..."
            class="flex-1 rounded-lg border px-3 py-2 dark:border-gray-600 dark:bg-gray-800 dark:text-white"
          />
          <button
            onClick={handleCreate}
            disabled={!newTodoContent().trim()}
            class="rounded-lg bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
            aria-label="Agregar tarea"
          >
            ➕ Agregar
          </button>
        </div>
      </Show>

      {/* Empty State */}
      <Show when={!loading() && todos().length === 0}>
        <div class="rounded-lg border border-dashed border-gray-300 p-8 text-center dark:border-gray-600">
          <p class="text-gray-500 dark:text-gray-400">
            No hay tareas aún. ¡Agrega tu primera tarea!
          </p>
        </div>
      </Show>

      {/* TODOs List */}
      <Show when={!loading() && todos().length > 0}>
        <div class="flex flex-col gap-4">
          {/* Pendientes */}
          <Show when={pendingTodos().length > 0}>
            <div>
              <h3 class="mb-2 text-sm font-semibold text-gray-700 dark:text-gray-300">
                📋 Pendientes ({pendingTodos().length})
              </h3>
              <div class="flex flex-col gap-2">
                <For each={pendingTodos()}>
                  {(todo) => (
                    <div class="flex items-center gap-2 rounded-lg border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800">
                      <input
                        type="checkbox"
                        checked={todo.is_completed}
                        onChange={() => handleToggle(todo)}
                        class="h-4 w-4 cursor-pointer"
                      />
                      <span class="flex-1 text-sm dark:text-white">
                        {todo.content}
                      </span>
                      <button
                        onClick={() => handleDelete(todo.id)}
                        class="text-red-600 hover:text-red-800 dark:text-red-400"
                        aria-label="Eliminar tarea"
                      >
                        🗑️
                      </button>
                    </div>
                  )}
                </For>
              </div>
            </div>
          </Show>

          {/* Completadas */}
          <Show when={completedTodos().length > 0}>
            <div>
              <h3 class="mb-2 text-sm font-semibold text-gray-700 dark:text-gray-300">
                ✅ Completadas ({completedTodos().length})
              </h3>
              <div class="flex flex-col gap-2">
                <For each={completedTodos()}>
                  {(todo) => (
                    <div class="flex items-center gap-2 rounded-lg border border-gray-200 bg-gray-50 p-3 dark:border-gray-700 dark:bg-gray-900">
                      <input
                        type="checkbox"
                        checked={todo.is_completed}
                        onChange={() => handleToggle(todo)}
                        class="h-4 w-4 cursor-pointer"
                      />
                      <span class="flex-1 text-sm text-gray-500 line-through dark:text-gray-400">
                        {todo.content}
                      </span>
                      <button
                        onClick={() => handleDelete(todo.id)}
                        class="text-red-600 hover:text-red-800 dark:text-red-400"
                        aria-label="Eliminar tarea"
                      >
                        🗑️
                      </button>
                    </div>
                  )}
                </For>
              </div>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}
