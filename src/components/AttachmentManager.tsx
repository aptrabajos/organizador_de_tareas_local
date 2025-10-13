/* eslint-disable no-undef */
import { Component, createSignal, For, Show, onMount } from 'solid-js';
import {
  addAttachment,
  getAttachments,
  deleteAttachment,
} from '../services/api';
import type { ProjectAttachment, CreateAttachmentDTO } from '../types/project';

interface AttachmentManagerProps {
  projectId: number;
}

const AttachmentManager: Component<AttachmentManagerProps> = (props) => {
  const [attachments, setAttachments] = createSignal<ProjectAttachment[]>([]);
  const [isUploading, setIsUploading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [isDragging, setIsDragging] = createSignal(false);
  const [previewImage, setPreviewImage] = createSignal<ProjectAttachment | null>(null);

  // Cargar adjuntos al montar el componente
  onMount(async () => {
    await loadAttachments();
  });

  const loadAttachments = async () => {
    try {
      const files = await getAttachments(props.projectId);
      setAttachments(files);
    } catch (err) {
      console.error('Error loading attachments:', err);
      setError('Error al cargar archivos adjuntos');
    }
  };

  const handleFileInput = async (e: Event) => {
    const input = e.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    const file = input.files[0];
    await uploadFile(file);

    // Limpiar input
    input.value = '';
  };

  const uploadFile = async (file: File) => {
    // Validar tamaño (5MB)
    const maxSize = 5 * 1024 * 1024;
    if (file.size > maxSize) {
      setError('El archivo excede el límite de 5MB');
      setTimeout(() => setError(null), 3000);
      return;
    }

    setIsUploading(true);
    setError(null);

    try {
      // Convertir archivo a base64
      const base64 = await fileToBase64(file);

      const attachmentData: CreateAttachmentDTO = {
        project_id: props.projectId,
        filename: file.name,
        file_data: base64,
        file_size: file.size,
        mime_type: file.type || 'application/octet-stream',
      };

      await addAttachment(attachmentData);
      await loadAttachments();
    } catch (err) {
      console.error('Error uploading file:', err);
      setError('Error al subir el archivo');
      setTimeout(() => setError(null), 3000);
    } finally {
      setIsUploading(false);
    }
  };

  const handleDelete = async (id: number, filename: string) => {
    if (!confirm(`¿Eliminar el archivo "${filename}"?`)) return;

    try {
      await deleteAttachment(id);
      await loadAttachments();
    } catch (err) {
      console.error('Error deleting attachment:', err);
      setError('Error al eliminar el archivo');
      setTimeout(() => setError(null), 3000);
    }
  };

  const downloadAttachment = (attachment: ProjectAttachment) => {
    try {
      // Convertir base64 a blob
      const byteCharacters = atob(attachment.file_data);
      const byteNumbers = new Array(byteCharacters.length);
      for (let i = 0; i < byteCharacters.length; i++) {
        byteNumbers[i] = byteCharacters.charCodeAt(i);
      }
      const byteArray = new Uint8Array(byteNumbers);
      const blob = new Blob([byteArray], { type: attachment.mime_type });

      // Crear link de descarga
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = attachment.filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Error downloading file:', err);
      setError('Error al descargar el archivo');
      setTimeout(() => setError(null), 3000);
    }
  };

  const fileToBase64 = (file: File): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.readAsDataURL(file);
      reader.onload = () => {
        const result = reader.result as string;
        // Remover el prefijo "data:mime/type;base64,"
        const base64 = result.split(',')[1];
        resolve(base64);
      };
      reader.onerror = (error) => reject(error);
    });
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const getFileIcon = (mimeType: string): string => {
    if (mimeType.startsWith('image/')) return '🖼️';
    if (mimeType.startsWith('video/')) return '🎥';
    if (mimeType.startsWith('audio/')) return '🎵';
    if (mimeType.includes('pdf')) return '📄';
    if (mimeType.includes('zip') || mimeType.includes('rar'))
      return '📦';
    if (
      mimeType.includes('word') ||
      mimeType.includes('document')
    )
      return '📝';
    if (
      mimeType.includes('sheet') ||
      mimeType.includes('excel')
    )
      return '📊';
    return '📎';
  };

  return (
    <div class="space-y-4">
      <div class="flex items-center justify-between">
        <h3 class="text-lg font-medium text-gray-900 dark:text-gray-100">
          Archivos Adjuntos
        </h3>
        <label
          class="inline-flex items-center px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors cursor-pointer disabled:opacity-50"
          classList={{ 'opacity-50 cursor-not-allowed': isUploading() }}
        >
          <span>{isUploading() ? 'Subiendo...' : 'Subir Archivo'}</span>
          <input
            type="file"
            class="hidden"
            onChange={handleFileInput}
            disabled={isUploading()}
          />
        </label>
      </div>

      <Show when={error()}>
        <div class="p-3 bg-red-100 dark:bg-red-900/20 text-red-700 dark:text-red-400 rounded-lg">
          {error()}
        </div>
      </Show>

      <div class="text-sm text-gray-600 dark:text-gray-400">
        Tamaño máximo: 5MB
      </div>

      <Show
        when={attachments().length > 0}
        fallback={
          <div class="text-center py-8 text-gray-500 dark:text-gray-400">
            No hay archivos adjuntos
          </div>
        }
      >
        <div class="space-y-2">
          <For each={attachments()}>
            {(attachment) => (
              <div class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors">
                <div class="flex items-center space-x-3 flex-1 min-w-0">
                  <span class="text-2xl">
                    {getFileIcon(attachment.mime_type)}
                  </span>
                  <div class="flex-1 min-w-0">
                    <div class="font-medium text-gray-900 dark:text-gray-100 truncate">
                      {attachment.filename}
                    </div>
                    <div class="text-sm text-gray-500 dark:text-gray-400">
                      {formatFileSize(attachment.file_size)} •{' '}
                      {new Date(attachment.created_at).toLocaleDateString()}
                    </div>
                  </div>
                </div>
                <div class="flex items-center space-x-2 ml-4">
                  <button
                    onClick={() => downloadAttachment(attachment)}
                    class="p-2 text-blue-600 hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-900/20 rounded transition-colors"
                    title="Descargar"
                  >
                    <svg
                      class="w-5 h-5"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                      />
                    </svg>
                  </button>
                  <button
                    onClick={() =>
                      handleDelete(attachment.id, attachment.filename)
                    }
                    class="p-2 text-red-600 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20 rounded transition-colors"
                    title="Eliminar"
                  >
                    <svg
                      class="w-5 h-5"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                      />
                    </svg>
                  </button>
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};

export default AttachmentManager;
