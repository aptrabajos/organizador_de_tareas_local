import { Component, createSignal, Show, For, onMount } from 'solid-js';
import { ProjectLink } from '../types/project';
import { createProjectLink, getProjectLinks, deleteProjectLink, openUrl } from '../services/api';
import type { CreateLinkDTO } from '../services/api';

interface ProjectLinksProps {
  projectId: number;
  projectName: string;
}

const LINK_TYPES = [
  { value: 'repository', label: 'Repositorio', icon: '📁', color: 'bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-200' },
  { value: 'documentation', label: 'Documentación', icon: '📚', color: 'bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-200' },
  { value: 'staging', label: 'Staging', icon: '🧪', color: 'bg-yellow-100 dark:bg-yellow-900/30 text-yellow-800 dark:text-yellow-200' },
  { value: 'production', label: 'Producción', icon: '🚀', color: 'bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-200' },
  { value: 'design', label: 'Diseño', icon: '🎨', color: 'bg-purple-100 dark:bg-purple-900/30 text-purple-800 dark:text-purple-200' },
  { value: 'api', label: 'API', icon: '🔌', color: 'bg-orange-100 dark:bg-orange-900/30 text-orange-800 dark:text-orange-200' },
  { value: 'other', label: 'Otro', icon: '🔗', color: 'bg-gray-100 dark:bg-gray-900/30 text-gray-800 dark:text-gray-200' },
] as const;

const ProjectLinks: Component<ProjectLinksProps> = (props) => {
  const [links, setLinks] = createSignal<ProjectLink[]>([]);
  const [showAddForm, setShowAddForm] = createSignal(false);
  const [isLoading, setIsLoading] = createSignal(false);
  const [newLink, setNewLink] = createSignal<Partial<CreateLinkDTO>>({
    project_id: props.projectId,
    link_type: 'repository',
    title: '',
    url: '',
  });

  const loadLinks = async () => {
    try {
      setIsLoading(true);
      const projectLinks = await getProjectLinks(props.projectId);
      setLinks(projectLinks);
    } catch (error) {
      console.error('Error cargando enlaces:', error);
    } finally {
      setIsLoading(false);
    }
  };

  onMount(() => {
    loadLinks();
  });

  const handleAddLink = async () => {
    const linkData = newLink();
    if (!linkData.title || !linkData.url) {
      alert('Por favor completa todos los campos');
      return;
    }

    try {
      setIsLoading(true);
      await createProjectLink(linkData as CreateLinkDTO);
      setNewLink({
        project_id: props.projectId,
        link_type: 'repository',
        title: '',
        url: '',
      });
      setShowAddForm(false);
      await loadLinks();
    } catch (error) {
      console.error('Error creando enlace:', error);
      alert('Error al crear el enlace');
    } finally {
      setIsLoading(false);
    }
  };

  const handleDeleteLink = async (linkId: number) => {
    if (!confirm('¿Estás seguro de que quieres eliminar este enlace?')) {
      return;
    }

    try {
      setIsLoading(true);
      await deleteProjectLink(linkId);
      await loadLinks();
    } catch (error) {
      console.error('Error eliminando enlace:', error);
      alert('Error al eliminar el enlace');
    } finally {
      setIsLoading(false);
    }
  };

  const handleOpenLink = async (url: string) => {
    try {
      await openUrl(url);
    } catch (error) {
      console.error('Error abriendo URL:', error);
      alert('Error al abrir el enlace');
    }
  };

  const getLinkTypeInfo = (type: string) => {
    return LINK_TYPES.find(t => t.value === type) || LINK_TYPES[6];
  };

  return (
    <div class="space-y-4">
      <div class="flex items-center justify-between">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
          Enlaces del Proyecto
        </h3>
        <button
          onClick={() => setShowAddForm(!showAddForm())}
          class="rounded-lg bg-blue-600 dark:bg-blue-500 px-3 py-1.5 text-sm text-white hover:bg-blue-700 dark:hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
        >
          + Agregar Enlace
        </button>
      </div>

      <Show when={showAddForm()}>
        <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 p-4">
          <h4 class="text-sm font-medium text-gray-900 dark:text-white mb-3">
            Nuevo Enlace
          </h4>
          <div class="space-y-3">
            <div>
              <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                Tipo
              </label>
              <select
                value={newLink().link_type || 'repository'}
                onChange={(e) => setNewLink({ ...newLink(), link_type: e.currentTarget.value as any })}
                class="w-full rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white px-2 py-1 text-sm"
              >
                <For each={LINK_TYPES}>
                  {(type) => (
                    <option value={type.value}>
                      {type.icon} {type.label}
                    </option>
                  )}
                </For>
              </select>
            </div>
            <div>
              <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                Título
              </label>
              <input
                type="text"
                value={newLink().title || ''}
                onInput={(e) => setNewLink({ ...newLink(), title: e.currentTarget.value })}
                placeholder="Ej: Repositorio principal"
                class="w-full rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white px-2 py-1 text-sm"
              />
            </div>
            <div>
              <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                URL
              </label>
              <input
                type="url"
                value={newLink().url || ''}
                onInput={(e) => setNewLink({ ...newLink(), url: e.currentTarget.value })}
                placeholder="https://github.com/usuario/proyecto"
                class="w-full rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white px-2 py-1 text-sm"
              />
            </div>
            <div class="flex space-x-2">
              <button
                onClick={handleAddLink}
                disabled={isLoading()}
                class="flex-1 rounded bg-blue-600 dark:bg-blue-500 px-3 py-1.5 text-sm text-white hover:bg-blue-700 dark:hover:bg-blue-600 disabled:opacity-50"
              >
                {isLoading() ? 'Guardando...' : 'Guardar'}
              </button>
              <button
                onClick={() => setShowAddForm(false)}
                class="flex-1 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 px-3 py-1.5 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-600"
              >
                Cancelar
              </button>
            </div>
          </div>
        </div>
      </Show>

      <Show when={isLoading() && links().length === 0}>
        <div class="text-center py-4">
          <p class="text-gray-500 dark:text-gray-400">Cargando enlaces...</p>
        </div>
      </Show>

      <Show when={links().length === 0 && !isLoading()}>
        <div class="text-center py-4 text-gray-500 dark:text-gray-400">
          <p class="text-sm">No hay enlaces agregados aún</p>
        </div>
      </Show>

      <div class="space-y-2">
        <For each={links()}>
          {(link) => {
            const typeInfo = getLinkTypeInfo(link.link_type);
            return (
              <div class="flex items-center justify-between rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-3">
                <div class="flex items-center space-x-3 flex-1">
                  <span class="text-lg">{typeInfo.icon}</span>
                  <div class="flex-1 min-w-0">
                    <p class="text-sm font-medium text-gray-900 dark:text-white truncate">
                      {link.title}
                    </p>
                    <p class="text-xs text-gray-500 dark:text-gray-400 truncate">
                      {link.url}
                    </p>
                  </div>
                  <span class={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${typeInfo.color}`}>
                    {typeInfo.label}
                  </span>
                </div>
                <div class="flex items-center space-x-2 ml-3">
                  <button
                    onClick={() => handleOpenLink(link.url)}
                    class="rounded p-1.5 text-gray-400 dark:text-gray-500 hover:text-blue-600 dark:hover:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/30"
                    title="Abrir enlace"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                    </svg>
                  </button>
                  <button
                    onClick={() => handleDeleteLink(link.id)}
                    class="rounded p-1.5 text-gray-400 dark:text-gray-500 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30"
                    title="Eliminar enlace"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                    </svg>
                  </button>
                </div>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
};

export default ProjectLinks;
