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
      return tagsStr.split(',').map((t) => t.trim()).filter(Boolean);
    }
  };

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-4xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div class="flex items-center justify-between p-4 border-b dark:border-gray-700">
          <h2 class="text-xl font-bold dark:text-white">📓 Diario del Proyecto</h2>
          <button
            onClick={() => props.onClose()}
            class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 text-2xl"
          >
            ×
          </button>
        </div>

        {/* Error Message */}
        <Show when={error()}>
          <div class="m-4 p-3 bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-200 rounded">
            {error()}
          </div>
        </Show>

        {/* New Entry Form */}
        <div class="p-4 border-b dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
          <textarea
            value={newContent()}
            onInput={(e) => setNewContent(e.currentTarget.value)}
            placeholder="Escribe una nota rápida... (soporta Markdown)"
            class="w-full p-3 border dark:border-gray-600 rounded-lg dark:bg-gray-800 dark:text-white resize-none"
            rows={3}
          />
          <div class="flex gap-2 mt-2">
            <input
              type="text"
              value={newTags()}
              onInput={(e) => setNewTags(e.currentTarget.value)}
              placeholder="Tags: bug, tip, idea (separados por coma)"
              class="flex-1 px-3 py-2 border dark:border-gray-600 rounded-lg dark:bg-gray-800 dark:text-white text-sm"
            />
            <button
              onClick={handleCreate}
              disabled={!newContent().trim()}
              class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              💾 Guardar
            </button>
          </div>
        </div>

        {/* Entries List */}
        <div class="flex-1 overflow-y-auto p-4 space-y-4">
          <Show when={loading()}>
            <div class="text-center py-8 text-gray-500 dark:text-gray-400">
              Cargando entradas...
            </div>
          </Show>

          <Show when={!loading() && entries().length === 0}>
            <div class="text-center py-8 text-gray-500 dark:text-gray-400">
              No hay entradas aún. ¡Crea la primera!
            </div>
          </Show>

          <For each={entries()}>
            {(entry) => (
              <div class="border dark:border-gray-700 rounded-lg p-4 bg-white dark:bg-gray-800">
                <Show
                  when={editingId() === entry.id}
                  fallback={
                    <>
                      {/* View Mode */}
                      <div class="flex items-start justify-between mb-2">
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
                            class="text-blue-600 hover:text-blue-800 dark:text-blue-400 text-sm"
                          >
                            ✏️
                          </button>
                          <button
                            onClick={() => handleDelete(entry.id)}
                            class="text-red-600 hover:text-red-800 dark:text-red-400 text-sm"
                          >
                            🗑️
                          </button>
                        </div>
                      </div>

                      {/* Rendered Markdown Content */}
                      <div
                        class="prose dark:prose-invert prose-sm max-w-none mb-3"
                        innerHTML={renderMarkdown(entry.content)}
                      />

                      {/* Tags */}
                      <Show when={entry.tags}>
                        <div class="flex flex-wrap gap-2">
                          <For each={parseTags(entry.tags)}>
                            {(tag) => (
                              <span class="px-2 py-1 bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200 text-xs rounded">
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
                    class="w-full p-3 border dark:border-gray-600 rounded-lg dark:bg-gray-900 dark:text-white resize-none mb-2"
                    rows={4}
                  />
                  <input
                    type="text"
                    value={editTags()}
                    onInput={(e) => setEditTags(e.currentTarget.value)}
                    placeholder="Tags separados por coma"
                    class="w-full px-3 py-2 border dark:border-gray-600 rounded-lg dark:bg-gray-900 dark:text-white text-sm mb-2"
                  />
                  <div class="flex gap-2">
                    <button
                      onClick={handleSaveEdit}
                      class="px-3 py-1 bg-green-600 text-white rounded hover:bg-green-700 text-sm"
                    >
                      ✓ Guardar
                    </button>
                    <button
                      onClick={handleCancelEdit}
                      class="px-3 py-1 bg-gray-600 text-white rounded hover:bg-gray-700 text-sm"
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
