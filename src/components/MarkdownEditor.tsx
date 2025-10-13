import { Component, createSignal, createEffect } from 'solid-js';
import { marked } from 'marked';
import DOMPurify from 'dompurify';

interface MarkdownEditorProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

const MarkdownEditor: Component<MarkdownEditorProps> = (props) => {
  const [activeTab, setActiveTab] = createSignal<'edit' | 'preview'>('edit');
  const [renderedHtml, setRenderedHtml] = createSignal('');

  // Configurar marked para soportar checkboxes
  marked.use({
    breaks: true,
    gfm: true,
  });

  // Renderizar markdown cuando cambia el valor o la tab
  createEffect(() => {
    if (activeTab() === 'preview') {
      let html = marked.parse(props.value || props.placeholder || '') as string;

      // Hacer que los checkboxes sean visibles con mejor estilo
      html = html.replace(
        /<input type="checkbox"(.*?)>/g,
        '<input type="checkbox" class="mr-2 h-4 w-4 cursor-pointer" onclick="return false;"$1>'
      );

      // Sanitizar HTML para prevenir XSS
      const sanitized = DOMPurify.sanitize(html);
      setRenderedHtml(sanitized);
    }
  });

  const handleTextareaChange = (e: Event) => {
    const target = e.target as HTMLTextAreaElement;
    props.onChange(target.value);
  };

  return (
    <div class="flex flex-col">
      {/* Tabs */}
      <div class="mb-2 flex gap-2 border-b border-gray-200 dark:border-gray-700">
        <button
          type="button"
          onClick={() => setActiveTab('edit')}
          class={`px-4 py-2 text-sm font-medium transition-colors ${
            activeTab() === 'edit'
              ? 'border-b-2 border-blue-600 text-blue-600 dark:border-blue-400 dark:text-blue-400'
              : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200'
          }`}
        >
          ✏️ Editar
        </button>
        <button
          type="button"
          onClick={() => setActiveTab('preview')}
          class={`px-4 py-2 text-sm font-medium transition-colors ${
            activeTab() === 'preview'
              ? 'border-b-2 border-blue-600 text-blue-600 dark:border-blue-400 dark:text-blue-400'
              : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200'
          }`}
        >
          👁️ Vista Previa
        </button>
      </div>

      {/* Editor */}
      {activeTab() === 'edit' && (
        <div class="relative">
          <textarea
            value={props.value}
            onInput={handleTextareaChange}
            placeholder={
              props.placeholder ||
              'Escribe en Markdown...\n\n# Título\n- [ ] Tarea pendiente\n- [x] Tarea completada\n**Negrita** *Cursiva* `código`'
            }
            class="w-full rounded-lg border border-gray-300 bg-white p-3 font-mono text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:placeholder-gray-500 dark:focus:border-blue-400 dark:focus:ring-blue-400"
            rows={12}
          />
          <div class="mt-2 text-xs text-gray-500 dark:text-gray-400">
            💡 Soporta Markdown: <strong>**negrita**</strong>,{' '}
            <em>*cursiva*</em>, <code>`código`</code>, listas, links, y más
          </div>
        </div>
      )}

      {/* Preview */}
      {activeTab() === 'preview' && (
        <div
          class="prose prose-sm max-w-none rounded-lg border border-gray-300 bg-gray-50 p-4 dark:prose-invert dark:border-gray-600 dark:bg-gray-800"
          innerHTML={renderedHtml()}
        />
      )}
    </div>
  );
};

export default MarkdownEditor;
