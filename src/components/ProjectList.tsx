import { Component, For, Show } from 'solid-js';
import type { Project } from '../types/project';
import { openUrl } from '../services/api';

interface ProjectListProps {
  projects: Project[];
  onEdit: (project: Project) => void;
  onDelete: (project: Project) => void;
  onOpenTerminal: (project: Project) => void;
}

const ProjectList: Component<ProjectListProps> = (props) => {
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
                  <a
                    href={project.documentation_url!}
                    target="_blank"
                    rel="noopener noreferrer"
                    onClick={(e) => {
                      e.preventDefault();
                      openUrl(project.documentation_url!);
                    }}
                    class="rounded bg-blue-100 px-2 py-1 text-xs text-blue-700 hover:bg-blue-200"
                  >
                    📖 Documentación
                  </a>
                </Show>
                <Show when={project.ai_documentation_url}>
                  <a
                    href={project.ai_documentation_url!}
                    target="_blank"
                    rel="noopener noreferrer"
                    onClick={(e) => {
                      e.preventDefault();
                      openUrl(project.ai_documentation_url!);
                    }}
                    class="rounded bg-purple-100 px-2 py-1 text-xs text-purple-700 hover:bg-purple-200"
                  >
                    🤖 Docs IA
                  </a>
                </Show>
                <Show when={project.drive_link}>
                  <a
                    href={project.drive_link!}
                    target="_blank"
                    rel="noopener noreferrer"
                    onClick={(e) => {
                      e.preventDefault();
                      openUrl(project.drive_link!);
                    }}
                    class="rounded bg-green-100 px-2 py-1 text-xs text-green-700 hover:bg-green-200"
                  >
                    📂 Drive
                  </a>
                </Show>
              </div>

              <div class="mt-4 flex gap-2">
                <button
                  onClick={() => props.onOpenTerminal(project)}
                  class="flex-1 rounded bg-purple-600 px-3 py-2 text-sm font-medium text-white hover:bg-purple-700"
                >
                  🚀 Trabajar
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
