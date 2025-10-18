import { Component, For, Show, createSignal } from 'solid-js';
import toast from 'solid-toast';
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import type { Project } from '../types/project';
import GitInfo from './GitInfo';
import ProjectJournal from './ProjectJournal';
import TodoList from './TodoList';
import ProjectContext from './ProjectContext';
import {
  openUrl,
  createProjectBackup,
  syncProjectToBackup,
  trackProjectOpen,
} from '../services/api';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { writeTextFile } from '@tauri-apps/plugin-fs';

// Configurar marked para soportar GFM y checkboxes
marked.use({
  breaks: true,
  gfm: true,
});

interface ProjectListProps {
  projects: Project[];
  onEdit: (project: Project) => void;
  onDelete: (project: Project) => void;
  onOpenTerminal: (project: Project) => void;
}

const ProjectList: Component<ProjectListProps> = (props) => {
  // Estado para controlar el journal modal
  const [journalProjectId, setJournalProjectId] = createSignal<number | null>(
    null
  );

  // Estado para controlar el modal de TODOs
  const [todosProjectId, setTodosProjectId] = createSignal<number | null>(null);

  // Estado para controlar el modal de contexto del proyecto
  const [contextProjectId, setContextProjectId] = createSignal<number | null>(
    null
  );

  // Estado para filtros
  const [statusFilter, setStatusFilter] = createSignal<string>('all');
  const [showPinnedOnly, setShowPinnedOnly] = createSignal(false);

  // Función para filtrar proyectos
  const filteredProjects = () => {
    let filtered = [...props.projects];

    // Filtrar por estado
    if (statusFilter() !== 'all') {
      filtered = filtered.filter((p) => p.status === statusFilter());
    }

    // Filtrar por pinned
    if (showPinnedOnly()) {
      filtered = filtered.filter((p) => p.is_pinned === true);
    }

    return filtered;
  };

  // Función helper para renderizar markdown de forma segura
  const renderMarkdown = (markdown: string): string => {
    let html = marked.parse(markdown) as string;

    // Estilizar checkboxes
    html = html.replace(
      /<input type="checkbox"(.*?)>/g,
      '<input type="checkbox" class="mr-1 h-3 w-3 cursor-pointer" onclick="return false;"$1>'
    );

    // Sanitizar HTML
    return DOMPurify.sanitize(html);
  };

  const handleOpenTerminal = async (project: Project) => {
    try {
      // Registrar apertura del proyecto para analytics
      await trackProjectOpen(project.id);
      console.log(`📊 Tracking registrado para proyecto: ${project.name}`);

      // Abrir terminal
      await props.onOpenTerminal(project);
    } catch (error) {
      console.error('Error al abrir proyecto:', error);
      // Aunque falle el tracking, abrimos el terminal igual
      await props.onOpenTerminal(project);
    }
  };

  const handleBackup = async (project: Project) => {
    try {
      // Pedir al usuario que seleccione la carpeta destino
      const destinationFolder = await open({
        directory: true,
        multiple: false,
        title: 'Selecciona carpeta donde guardar el backup',
        defaultPath: project.local_path, // Sugerencia: carpeta del proyecto
      });

      if (!destinationFolder) {
        return; // Usuario canceló
      }

      const toastId = toast.loading('Creando backup...');
      try {
        const backupData = await createProjectBackup(project.id);
        // Construir ruta completa con la carpeta elegida
        const fullPath = `${destinationFolder}/${backupData.filename}`;
        await writeTextFile(fullPath, backupData.content);
        toast.success(`✅ Backup creado en:\n${fullPath}`, {
          id: toastId,
          duration: 5000,
        });
      } catch (error) {
        console.error('Error al crear backup:', error);
        toast.error(`Error al crear backup: ${error}`, { id: toastId });
      }
    } catch (error) {
      console.error('Error abriendo selector:', error);
      toast.error(`Error al abrir selector de carpetas: ${error}`);
    }
  };

  const handleBackupToMnt = async (project: Project) => {
    try {
      // Crear carpeta del proyecto en /mnt/sda1 y guardar backup ahí
      const projectFolder = `/mnt/sda1/${project.name}`;
      const toastId = toast.loading(`Creando backup en ${projectFolder}...`);

      try {
        const backupData = await createProjectBackup(project.id);
        const fullPath = `${projectFolder}/${backupData.filename}`;

        // Usar el comando personalizado de Tauri (crea la carpeta automáticamente)
        await invoke('write_file_to_path', {
          filePath: fullPath,
          content: backupData.content,
        });

        toast.success(`✅ Backup creado en:\n${fullPath}`, {
          id: toastId,
          duration: 5000,
        });
      } catch (error) {
        console.error('Error al crear backup:', error);
        toast.error(`Error al crear backup: ${error}`, { id: toastId });
      }
    } catch (error) {
      console.error('Error:', error);
      toast.error(`Error: ${error}`);
    }
  };

  const handleSync = async (project: Project) => {
    try {
      // Sincronizar directamente al backup en /mnt/sda1 con rsync
      const toastId = toast.loading(
        `Sincronizando ${project.name} con rsync...`
      );

      try {
        const result = await syncProjectToBackup(
          project.local_path,
          project.name
        );
        toast.success(`✅ ${result}`, {
          id: toastId,
          duration: 5000,
        });
      } catch (error) {
        console.error('Error al sincronizar con rsync:', error);
        toast.error(`Error al sincronizar: ${error}`, { id: toastId });
      }
    } catch (error) {
      console.error('Error:', error);
      toast.error(`Error: ${error}`);
    }
  };

  return (
    <Show
      when={props.projects.length > 0}
      fallback={
        <div class="py-12 text-center text-gray-500 dark:text-gray-400">
          <p class="text-lg">No hay proyectos disponibles</p>
          <p class="mt-2 text-sm">Crea tu primer proyecto para comenzar</p>
        </div>
      }
    >
      <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        <For each={props.projects}>
          {(project) => (
            <div class="rounded-lg border border-gray-200 bg-white p-4 shadow-sm transition-shadow hover:shadow-md dark:border-gray-700 dark:bg-gray-800">
              <div class="flex gap-3">
                <Show when={project.image_data}>
                  <img
                    src={project.image_data}
                    alt={project.name}
                    class="h-16 w-16 flex-shrink-0 rounded-lg border-2 border-gray-300 object-cover dark:border-gray-600"
                  />
                </Show>
                <div class="flex-1">
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                    {project.name}
                  </h3>
                  <p class="mt-1 text-sm text-gray-600 dark:text-gray-300">
                    {project.description}
                  </p>
                </div>
              </div>

              <div class="mt-3 space-y-2 text-xs text-gray-500 dark:text-gray-400">
                <p class="truncate" title={project.local_path}>
                  📁 {project.local_path}
                </p>
                <GitInfo projectPath={project.local_path} />
              </div>

              <Show when={project.notes}>
                <div class="mt-3 rounded bg-gray-50 p-2 text-xs text-gray-700 dark:bg-gray-700 dark:text-gray-300">
                  <p class="font-semibold">📝 Notas:</p>
                  <div
                    class="prose-xs prose mt-1 max-h-40 max-w-none overflow-y-auto break-words dark:prose-invert"
                    // eslint-disable-next-line solid/no-innerhtml
                    innerHTML={renderMarkdown(project.notes!)}
                  />
                </div>
              </Show>

              <div class="mt-3 flex flex-wrap gap-2">
                <Show when={project.documentation_url}>
                  <button
                    onClick={() => openUrl(project.documentation_url!)}
                    class="rounded bg-blue-100 px-2 py-1 text-xs text-blue-700 hover:bg-blue-200"
                    type="button"
                  >
                    📖 Documentación
                  </button>
                </Show>
                <Show when={project.ai_documentation_url}>
                  <button
                    onClick={() => openUrl(project.ai_documentation_url!)}
                    class="rounded bg-purple-100 px-2 py-1 text-xs text-purple-700 hover:bg-purple-200"
                    type="button"
                  >
                    🤖 Docs IA
                  </button>
                </Show>
                <Show when={project.drive_link}>
                  <button
                    onClick={() => openUrl(project.drive_link!)}
                    class="rounded bg-green-100 px-2 py-1 text-xs text-green-700 hover:bg-green-200"
                    type="button"
                  >
                    📂 Drive
                  </button>
                </Show>
              </div>

              <div class="mt-4 flex flex-wrap gap-2">
                <button
                  onClick={() => handleOpenTerminal(project)}
                  class="flex-1 rounded bg-purple-600 px-3 py-2 text-sm font-medium text-white hover:bg-purple-700"
                >
                  🚀 Trabajar
                </button>
                <button
                  onClick={() => setContextProjectId(project.id)}
                  class="rounded bg-cyan-600 px-3 py-2 text-sm font-medium text-white hover:bg-cyan-700"
                  aria-label="Contexto"
                  title="Ver contexto del proyecto"
                >
                  📋
                </button>
                <button
                  onClick={() => setJournalProjectId(project.id)}
                  class="rounded bg-amber-600 px-3 py-2 text-sm font-medium text-white hover:bg-amber-700"
                  aria-label="Diario"
                  title="Diario del proyecto"
                >
                  📓
                </button>
                <button
                  onClick={() => setTodosProjectId(project.id)}
                  class="rounded bg-green-600 px-3 py-2 text-sm font-medium text-white hover:bg-green-700"
                  aria-label="TODOs"
                  title="Lista de tareas"
                >
                  ✅
                </button>
                <button
                  onClick={() => handleBackup(project)}
                  class="rounded bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700"
                  aria-label="Crear backup"
                  title="Crear backup - Elegir carpeta"
                >
                  💾
                </button>
                <button
                  onClick={() => handleBackupToMnt(project)}
                  class="rounded bg-indigo-600 px-3 py-2 text-sm font-medium text-white hover:bg-indigo-700"
                  aria-label="Backup a disco"
                  title="Backup directo a /mnt/sda1"
                >
                  💿
                </button>
                <button
                  onClick={() => handleSync(project)}
                  class="rounded bg-green-600 px-3 py-2 text-sm font-medium text-white hover:bg-green-700"
                  aria-label="Sincronizar"
                  title="Sincronizar con rsync"
                >
                  🔄
                </button>
                <button
                  onClick={() => props.onEdit(project)}
                  class="rounded border border-gray-300 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50"
                  aria-label="Editar"
                >
                  ✏️
                </button>
                <button
                  onClick={() => props.onDelete(project)}
                  class="rounded border border-red-300 px-3 py-2 text-sm text-red-600 hover:bg-red-50"
                  aria-label="Eliminar"
                >
                  🗑️
                </button>
              </div>
            </div>
          )}
        </For>
      </div>

      {/* Journal Modal */}
      <Show when={journalProjectId() !== null}>
        <ProjectJournal
          projectId={journalProjectId()!}
          onClose={() => setJournalProjectId(null)}
        />
      </Show>

      {/* TODOs Modal */}
      <Show when={todosProjectId() !== null}>
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
          <div class="flex max-h-[90vh] w-full max-w-2xl flex-col rounded-lg bg-white shadow-xl dark:bg-gray-800">
            {/* Header */}
            <div class="flex items-center justify-between border-b p-4 dark:border-gray-700">
              <h2 class="text-xl font-bold dark:text-white">
                ✅ Lista de Tareas
              </h2>
              <button
                onClick={() => setTodosProjectId(null)}
                class="text-2xl text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
              >
                ×
              </button>
            </div>

            {/* Content */}
            <div class="flex-1 overflow-y-auto p-4">
              <TodoList projectId={todosProjectId()!} />
            </div>
          </div>
        </div>
      </Show>

      {/* Project Context Modal */}
      <Show when={contextProjectId() !== null}>
        <ProjectContext
          projectId={contextProjectId()!}
          onClose={() => setContextProjectId(null)}
        />
      </Show>
    </Show>
  );
};

export default ProjectList;
