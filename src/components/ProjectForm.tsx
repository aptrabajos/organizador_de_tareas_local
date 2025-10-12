import { Component, createSignal, Show } from 'solid-js';
import type { Project } from '../types/project';

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
  const initialProject = props.project;
  const [name, setName] = createSignal(initialProject?.name || '');
  const [description, setDescription] = createSignal(
    initialProject?.description || ''
  );
  const [localPath, setLocalPath] = createSignal(
    initialProject?.local_path || ''
  );
  const [documentationUrl, setDocumentationUrl] = createSignal(
    initialProject?.documentation_url || ''
  );
  const [aiDocumentationUrl, setAiDocumentationUrl] = createSignal(
    initialProject?.ai_documentation_url || ''
  );
  const [driveLink, setDriveLink] = createSignal(
    initialProject?.drive_link || ''
  );
  const [notes, setNotes] = createSignal(initialProject?.notes || '');
  const [imageData, setImageData] = createSignal(
    initialProject?.image_data || ''
  );
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

    const formData = {
      name: name().trim(),
      description: description().trim(),
      local_path: localPath().trim(),
      documentation_url: documentationUrl().trim(),
      ai_documentation_url: aiDocumentationUrl().trim(),
      drive_link: driveLink().trim(),
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
