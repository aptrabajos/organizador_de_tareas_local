import { createSignal, createEffect, For, Show } from 'solid-js';
import type { JournalEntry } from '../types/project';
import {
  createJournalEntry,
  getJournalEntries,
  updateJournalEntry,
  deleteJournalEntry,
} from '../services/api';
import { marked } from 'marked';
import DOMPurify from 'dompurify';

interface ProjectJournalProps {
  projectId: number;
  onClose: () => void;
}

export default function ProjectJournal(props: ProjectJournalProps) {
  const [entries, setEntries] = createSignal<JournalEntry[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [newContent, setNewContent] = createSignal('');
  const [newTags, setNewTags] = createSignal('');
  const [editingId, setEditingId] = createSignal<number | null>(null);
  const [editContent, setEditContent] = createSignal('');
  const [editTags, setEditTags] = createSignal('');
  const [error, setError] = createSignal<string | null>(null);

  // Cargar entradas al montar
  createEffect(() => {
    loadEntries();
  });

  const loadEntries = async () => {
    try {
      setLoading(true);
      const data = await getJournalEntries(props.projectId);
      setEntries(data);
      setError(null);
    } catch (err) {
      console.error('Error loading journal entries:', err);
      setError('Error al cargar las entradas del diario');
    } finally {
      setLoading(false);
    }
  };

  const handleCreate = async () => {
    const content = newContent().trim();
    if (!content) return;

    try {
      const tags = newTags().trim() || undefined;
      await createJournalEntry({
        project_id: props.projectId,
        content,
        tags,
      });
      setNewContent('');
      setNewTags('');
      await loadEntries();
    } catch (err) {
      console.error('Error creating journal entry:', err);
      setError('Error al crear entrada');
    }
  };

  const handleStartEdit = (entry: JournalEntry) => {
    setEditingId(entry.id);
    setEditContent(entry.content);
    setEditTags(entry.tags || '');
  };

  const handleSaveEdit = async () => {
    const id = editingId();
    if (id === null) return;

    try {
      await updateJournalEntry(id, {
        content: editContent().trim() || undefined,
        tags: editTags().trim() || undefined,
      });
      setEditingId(null);
      await loadEntries();
    } catch (err) {
      console.error('Error updating journal entry:', err);
      setError('Error al actualizar entrada');
    }
  };

  const handleCancelEdit = () => {
    setEditingId(null);
    setEditContent('');
    setEditTags('');
  };

  const handleDelete = async (id: number) => {
    if (!confirm('¿Eliminar esta entrada del diario?')) return;

    try {
      await deleteJournalEntry(id);
      await loadEntries();
    } catch (err) {
      console.error('Error deleting journal entry:', err);
      setError('Error al eliminar entrada');
    }
  };

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleString('es-AR', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const renderMarkdown = (content: string) => {
    const html = marked.parse(content, { async: false }) as string;
    return DOMPurify.sanitize(html);
  };

  const parseTags = (tagsStr?: string): string[] => {
    if (!tagsStr) return [];
    try {
      return JSON.parse(tagsStr);
    } catch {
      return tagsStr
        .split(',')
        .map((t) => t.trim())
        .filter(Boolean);
    }
  };

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
      <div class="flex max-h-[90vh] w-full max-w-4xl flex-col rounded-lg bg-white shadow-xl dark:bg-gray-800">
        {/* Header */}
        <div class="flex items-center justify-between border-b p-4 dark:border-gray-700">
          <h2 class="text-xl font-bold dark:text-white">
            📓 Diario del Proyecto
          </h2>
          <button
            onClick={() => props.onClose()}
            class="text-2xl text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
          >
            ×
          </button>
        </div>

        {/* Error Message */}
        <Show when={error()}>
          <div class="m-4 rounded bg-red-100 p-3 text-red-700 dark:bg-red-900 dark:text-red-200">
            {error()}
          </div>
        </Show>

        {/* New Entry Form */}
        <div class="border-b bg-gray-50 p-4 dark:border-gray-700 dark:bg-gray-900">
          <textarea
            value={newContent()}
            onInput={(e) => setNewContent(e.currentTarget.value)}
            placeholder="Escribe una nota rápida... (soporta Markdown)"
            class="w-full resize-none rounded-lg border p-3 dark:border-gray-600 dark:bg-gray-800 dark:text-white"
            rows={3}
          />
          <div class="mt-2 flex gap-2">
            <input
              type="text"
              value={newTags()}
              onInput={(e) => setNewTags(e.currentTarget.value)}
              placeholder="Tags: bug, tip, idea (separados por coma)"
              class="flex-1 rounded-lg border px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-800 dark:text-white"
            />
            <button
              onClick={handleCreate}
              disabled={!newContent().trim()}
              class="rounded-lg bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
            >
              💾 Guardar
            </button>
          </div>
        </div>

        {/* Entries List */}
        <div class="flex-1 space-y-4 overflow-y-auto p-4">
          <Show when={loading()}>
            <div class="py-8 text-center text-gray-500 dark:text-gray-400">
              Cargando entradas...
            </div>
          </Show>

          <Show when={!loading() && entries().length === 0}>
            <div class="py-8 text-center text-gray-500 dark:text-gray-400">
              No hay entradas aún. ¡Crea la primera!
            </div>
          </Show>

          <For each={entries()}>
            {(entry) => (
              <div class="rounded-lg border bg-white p-4 dark:border-gray-700 dark:bg-gray-800">
                <Show
                  when={editingId() === entry.id}
                  fallback={
                    <>
                      {/* View Mode */}
                      <div class="mb-2 flex items-start justify-between">
                        <div class="text-sm text-gray-500 dark:text-gray-400">
                          📅 {formatDate(entry.created_at)}
                          <Show when={entry.created_at !== entry.updated_at}>
                            {' '}
                            <span class="italic">(editado)</span>
                          </Show>
                        </div>
                        <div class="flex gap-2">
                          <button
                            onClick={() => handleStartEdit(entry)}
                            class="text-sm text-blue-600 hover:text-blue-800 dark:text-blue-400"
                          >
                            ✏️
                          </button>
                          <button
                            onClick={() => handleDelete(entry.id)}
                            class="text-sm text-red-600 hover:text-red-800 dark:text-red-400"
                          >
                            🗑️
                          </button>
                        </div>
                      </div>

                      {/* Rendered Markdown Content - sanitized with DOMPurify */}
                      <div
                        class="prose prose-sm mb-3 max-w-none dark:prose-invert"
                        // eslint-disable-next-line solid/no-innerhtml
                        innerHTML={renderMarkdown(entry.content)}
                      />

                      {/* Tags */}
                      <Show when={entry.tags}>
                        <div class="flex flex-wrap gap-2">
                          <For each={parseTags(entry.tags)}>
                            {(tag) => (
                              <span class="rounded bg-blue-100 px-2 py-1 text-xs text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                                #{tag}
                              </span>
                            )}
                          </For>
                        </div>
                      </Show>
                    </>
                  }
                >
                  {/* Edit Mode */}
                  <textarea
                    value={editContent()}
                    onInput={(e) => setEditContent(e.currentTarget.value)}
                    class="mb-2 w-full resize-none rounded-lg border p-3 dark:border-gray-600 dark:bg-gray-900 dark:text-white"
                    rows={4}
                  />
                  <input
                    type="text"
                    value={editTags()}
                    onInput={(e) => setEditTags(e.currentTarget.value)}
                    placeholder="Tags separados por coma"
                    class="mb-2 w-full rounded-lg border px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-900 dark:text-white"
                  />
                  <div class="flex gap-2">
                    <button
                      onClick={handleSaveEdit}
                      class="rounded bg-green-600 px-3 py-1 text-sm text-white hover:bg-green-700"
                    >
                      ✓ Guardar
                    </button>
                    <button
                      onClick={handleCancelEdit}
                      class="rounded bg-gray-600 px-3 py-1 text-sm text-white hover:bg-gray-700"
                    >
                      ✗ Cancelar
                    </button>
                  </div>
                </Show>
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  );
}
