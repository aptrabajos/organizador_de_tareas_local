import { Component, createSignal, Show } from 'solid-js';
import type { Project } from '../types/project';
import MarkdownEditor from './MarkdownEditor';

interface ProjectFormProps {
  project?: Project;
  onSubmit: (data: ProjectFormData) => void;
  onCancel: () => void;
}

export interface ProjectFormData {
  name: string;
  description: string;
  local_path: string;
  documentation_url?: string;
  ai_documentation_url?: string;
  drive_link?: string;
  notes?: string;
  image_data?: string;
}

const ProjectForm: Component<ProjectFormProps> = (props) => {
  // Capturar valores iniciales una sola vez (props.project no cambia después de montar)
  const initialValues = (() => {
    // eslint-disable-next-line solid/reactivity
    const p = props.project;
    return {
      name: p?.name || '',
      description: p?.description || '',
      localPath: p?.local_path || '',
      documentationUrl: p?.documentation_url || '',
      aiDocumentationUrl: p?.ai_documentation_url || '',
      driveLink: p?.drive_link || '',
      notes: p?.notes || '',
      imageData: p?.image_data || '',
    };
  })();

  const [name, setName] = createSignal(initialValues.name);
  const [description, setDescription] = createSignal(initialValues.description);
  const [localPath, setLocalPath] = createSignal(initialValues.localPath);
  const [documentationUrl, setDocumentationUrl] = createSignal(
    initialValues.documentationUrl
  );
  const [aiDocumentationUrl, setAiDocumentationUrl] = createSignal(
    initialValues.aiDocumentationUrl
  );
  const [driveLink, setDriveLink] = createSignal(initialValues.driveLink);
  const [notes, setNotes] = createSignal(initialValues.notes);
  const [imageData, setImageData] = createSignal(initialValues.imageData);
  const [imageError, setImageError] = createSignal('');

  const handleImageChange = async (e: Event) => {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];

    if (!file) return;

    // Validar tipo
    if (!file.type.startsWith('image/')) {
      setImageError('Por favor selecciona una imagen válida');
      return;
    }

    // Validar tamaño (máx 500KB)
    if (file.size > 500 * 1024) {
      setImageError('La imagen debe ser menor a 500KB');
      return;
    }

    setImageError('');

    // Leer y comprimir imagen
    const reader = new FileReader();
    reader.onload = (event) => {
      const img = new Image();
      img.onload = () => {
        // Crear canvas para redimensionar
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');

        if (!ctx) return;

        // Calcular dimensiones (máx 200x200, mantener aspecto)
        let width = img.width;
        let height = img.height;
        const maxSize = 200;

        if (width > height) {
          if (width > maxSize) {
            height = (height * maxSize) / width;
            width = maxSize;
          }
        } else {
          if (height > maxSize) {
            width = (width * maxSize) / height;
            height = maxSize;
          }
        }

        canvas.width = width;
        canvas.height = height;
        ctx.drawImage(img, 0, 0, width, height);

        // Convertir a base64 con compresión
        const base64 = canvas.toDataURL('image/jpeg', 0.7);
        setImageData(base64);
      };
      img.src = event.target?.result as string;
    };
    reader.readAsDataURL(file);
  };

  const removeImage = () => {
    setImageData('');
    setImageError('');
  };

  const handleSubmit = (e: SubmitEvent) => {
    e.preventDefault();

    console.log('🔧 [FRONTEND] Formulario enviado - INICIO');
    console.log('🔧 [FRONTEND] Evento:', e);
    console.log('🔧 [FRONTEND] Props:', props);
    console.log('📝 [FRONTEND] Datos del formulario:', {
      name: name(),
      description: description(),
      local_path: localPath(),
      documentation_url: documentationUrl(),
      ai_documentation_url: aiDocumentationUrl(),
      drive_link: driveLink(),
    });

    if (!name().trim() || !description().trim() || !localPath().trim()) {
      console.log('❌ [FRONTEND] Validación falló - campos requeridos vacíos');
      return;
    }

    const formData: ProjectFormData = {
      name: name().trim(),
      description: description().trim(),
      local_path: localPath().trim(),
      documentation_url: documentationUrl().trim() || undefined,
      ai_documentation_url: aiDocumentationUrl().trim() || undefined,
      drive_link: driveLink().trim() || undefined,
      notes: notes().trim() || undefined,
      image_data: imageData() || undefined,
    };

    console.log('✅ [FRONTEND] Enviando datos:', formData);
    props.onSubmit(formData);
  };

  return (
    <form onSubmit={handleSubmit} class="space-y-4">
      <div>
        <label
          for="name"
          class="block text-sm font-medium text-gray-700 dark:text-gray-300"
        >
          Nombre *
        </label>
        <input
          id="name"
          type="text"
          value={name()}
          onInput={(e) => setName(e.currentTarget.value)}
          class="mt-1 w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-200 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:focus:border-blue-400 dark:focus:ring-blue-800"
          required
        />
      </div>

      <div>
        <label
          for="description"
          class="block text-sm font-medium text-gray-700 dark:text-gray-300"
        >
          Descripción *
        </label>
        <textarea
          id="description"
          value={description()}
          onInput={(e) => setDescription(e.currentTarget.value)}
          rows={3}
          class="mt-1 w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-200 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:focus:border-blue-400 dark:focus:ring-blue-800"
          required
        />
      </div>

      <div>
        <label
          for="local_path"
          class="block text-sm font-medium text-gray-700 dark:text-gray-300"
        >
          Ruta Local *
        </label>
        <input
          id="local_path"
          type="text"
          value={localPath()}
          onInput={(e) => setLocalPath(e.currentTarget.value)}
          class="mt-1 w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-200 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:focus:border-blue-400 dark:focus:ring-blue-800"
          placeholder="/home/usuario/proyecto"
          required
        />
      </div>

      <div>
        <label
          for="documentation_url"
          class="block text-sm font-medium text-gray-700 dark:text-gray-300"
        >
          URL Documentación
        </label>
        <input
          id="documentation_url"
          type="url"
          value={documentationUrl()}
          onInput={(e) => setDocumentationUrl(e.currentTarget.value)}
          class="mt-1 w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-200 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:focus:border-blue-400 dark:focus:ring-blue-800"
          placeholder="https://docs.ejemplo.com"
        />
      </div>

      <div>
        <label
          for="ai_documentation_url"
          class="block text-sm font-medium text-gray-700 dark:text-gray-300"
        >
          URL Documentación IA
        </label>
        <input
          id="ai_documentation_url"
          type="url"
          value={aiDocumentationUrl()}
          onInput={(e) => setAiDocumentationUrl(e.currentTarget.value)}
          class="mt-1 w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-200 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:focus:border-blue-400 dark:focus:ring-blue-800"
          placeholder="https://ai-docs.ejemplo.com"
        />
      </div>

      <div>
        <label
          for="drive_link"
          class="block text-sm font-medium text-gray-700 dark:text-gray-300"
        >
          Link Google Drive
        </label>
        <input
          id="drive_link"
          type="url"
          value={driveLink()}
          onInput={(e) => setDriveLink(e.currentTarget.value)}
          class="mt-1 w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-200 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:focus:border-blue-400 dark:focus:ring-blue-800"
          placeholder="https://drive.google.com/..."
        />
      </div>

      <div>
        <label
          for="notes"
          class="block text-sm font-medium text-gray-700 dark:text-gray-300"
        >
          Notas
        </label>
        <textarea
          id="notes"
          value={notes()}
          onInput={(e) => setNotes(e.currentTarget.value)}
          rows={4}
          class="mt-1 w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-200 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:focus:border-blue-400 dark:focus:ring-blue-800"
          placeholder="Notas adicionales sobre el proyecto..."
        />
      </div>

      <div>
        <label
          for="image"
          class="block text-sm font-medium text-gray-700 dark:text-gray-300"
        >
          Imagen del Proyecto
        </label>
        <div class="mt-1 space-y-2">
          <Show when={imageData()}>
            <div class="relative inline-block">
              <img
                src={imageData()}
                alt="Preview"
                class="h-32 w-32 rounded-lg border-2 border-gray-300 object-cover dark:border-gray-600"
              />
              <button
                type="button"
                onClick={removeImage}
                class="absolute -right-2 -top-2 rounded-full bg-red-500 p-1 text-white hover:bg-red-600 focus:outline-none focus:ring-2 focus:ring-red-500"
                aria-label="Eliminar imagen"
              >
                <svg
                  class="h-4 w-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </button>
            </div>
          </Show>
          <input
            id="image"
            type="file"
            accept="image/*"
            onChange={handleImageChange}
            class="block w-full text-sm text-gray-500 file:mr-4 file:rounded-lg file:border-0 file:bg-blue-50 file:px-4 file:py-2 file:text-sm file:font-semibold file:text-blue-700 hover:file:bg-blue-100 dark:text-gray-400 dark:file:bg-blue-900 dark:file:text-blue-300 dark:hover:file:bg-blue-800"
          />
          <Show when={imageError()}>
            <p class="text-sm text-red-600 dark:text-red-400">{imageError()}</p>
          </Show>
          <p class="text-xs text-gray-500 dark:text-gray-400">
            Máximo 500KB, se redimensionará a 200x200px
          </p>
        </div>
      </div>

      <div class="flex gap-3 pt-4">
        <button
          type="submit"
          class="flex-1 rounded-lg bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-gray-800"
        >
          Guardar
        </button>
        <button
          type="button"
          onClick={() => props.onCancel()}
          class="flex-1 rounded-lg border border-gray-300 bg-white px-4 py-2 text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-300 dark:hover:bg-gray-600 dark:focus:ring-offset-gray-800"
        >
          Cancelar
        </button>
      </div>
    </form>
  );
};

export default ProjectForm;
