import { Component, For, Show } from 'solid-js';
import toast from 'solid-toast';
import type { Project } from '../types/project';
import { openUrl, createProjectBackup, syncProject } from '../services/api';
import { open } from '@tauri-apps/plugin-dialog';

interface ProjectListProps {
  projects: Project[];
  onEdit: (project: Project) => void;
  onDelete: (project: Project) => void;
  onOpenTerminal: (project: Project) => void;
}

const ProjectList: Component<ProjectListProps> = (props) => {
  const handleBackup = async (project: Project) => {
    try {
      const backupPath = await createProjectBackup(project.id);
      alert(`Backup creado exitosamente en:\n${backupPath}`);
    } catch (error) {
      alert(`Error al crear backup: ${error}`);
    }
  };

  const handleSync = async (project: Project) => {
    try {
      const destinationPath = await open({
        directory: true,
        multiple: false,
        title: 'Selecciona carpeta de destino para sincronización',
      });

      if (!destinationPath) {
        return; // Usuario canceló
      }

      const result = await syncProject(
        project.local_path,
        destinationPath as string
      );
      alert(result);
    } catch (error) {
      alert(`Error al sincronizar: ${error}`);
    }
  };

  return (
    <Show
      when={props.projects.length > 0}
      fallback={
        <div class="py-12 text-center text-gray-500">
          <p class="text-lg">No hay proyectos disponibles</p>
          <p class="mt-2 text-sm">Crea tu primer proyecto para comenzar</p>
        </div>
      }
    >
      <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        <For each={props.projects}>
          {(project) => (
            <div class="rounded-lg border border-gray-200 bg-white p-4 shadow-sm transition-shadow hover:shadow-md">
              <h3 class="text-lg font-semibold text-gray-900">
                {project.name}
              </h3>
              <p class="mt-1 text-sm text-gray-600">{project.description}</p>

              <div class="mt-3 space-y-1 text-xs text-gray-500">
                <p class="truncate" title={project.local_path}>
                  📁 {project.local_path}
                </p>
              </div>

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
                  onClick={() => props.onOpenTerminal(project)}
                  class="flex-1 rounded bg-purple-600 px-3 py-2 text-sm font-medium text-white hover:bg-purple-700"
                >
                  🚀 Trabajar
                </button>
                <button
                  onClick={() => handleBackup(project)}
                  class="rounded bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700"
                  aria-label="Crear backup"
                  title="Crear backup en Markdown"
                >
                  💾
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
    </Show>
  );
};

export default ProjectList;
