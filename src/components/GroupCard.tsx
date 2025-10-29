import { Component, createSignal, onMount, For, Show } from 'solid-js';
import { createDroppable } from '@thisbeyond/solid-dnd';
import type { Project } from '../types/project';
import { countSubprojects, getSubprojects } from '../services/api';

interface GroupCardProps {
  project: Project;
  onViewProjects: (project: Project) => void;
  onEdit: (project: Project) => void;
  onDelete: (project: Project) => void;
  onDropProject?: (projectId: number, groupId: number) => void; // v0.4.0 - Drag & Drop
}

const GroupCard: Component<GroupCardProps> = (props) => {
  const [subprojectCount, setSubprojectCount] = createSignal(0);
  const [showSubprojects, setShowSubprojects] = createSignal(false);
  const [subprojects, setSubprojects] = createSignal<Project[]>([]);
  const [loadingSubprojects, setLoadingSubprojects] = createSignal(false);

  // v0.4.0 - Hacer el GroupCard droppable para drag & drop
  const droppable = createDroppable(props.project.id);

  onMount(async () => {
    try {
      const count = await countSubprojects(props.project.id);
      setSubprojectCount(count);
    } catch (err) {
      console.error('Error counting subprojects:', err);
    }
  });

  // Cargar subproyectos cuando se expande la lista
  const toggleSubprojects = async () => {
    if (!showSubprojects() && subprojects().length === 0) {
      setLoadingSubprojects(true);
      try {
        const projects = await getSubprojects(props.project.id);
        setSubprojects(projects);
      } catch (err) {
        console.error('Error loading subprojects:', err);
      } finally {
        setLoadingSubprojects(false);
      }
    }
    setShowSubprojects(!showSubprojects());
  };

  // Color por defecto si no hay group_color
  const borderColor = () => props.project.group_color || '#3B82F6';
  const hasSubprojects = () => subprojectCount() > 0;

  return (
    <div
      ref={droppable.ref}
      class="flex h-full min-h-[280px] flex-col rounded-lg border-2 bg-white p-2 shadow-sm transition-all hover:shadow-md dark:bg-gray-800"
      classList={{
        'ring-4 ring-offset-2 scale-105': droppable.isActiveDroppable, // Feedback visual al arrastrar sobre el grupo
      }}
      style={{
        'border-color': borderColor(),
        '--tw-ring-color': droppable.isActiveDroppable
          ? borderColor()
          : undefined,
      }}
    >
      {/* Header con icono del grupo */}
      <div class="mb-2 flex items-start justify-between">
        <div class="flex items-center gap-2">
          <span class="text-3xl">{props.project.group_icon || '📁'}</span>
          <div class="flex-1">
            <h3 class="line-clamp-2 text-base font-semibold text-gray-900 dark:text-white">
              {props.project.name}
            </h3>
          </div>
        </div>
        <div class="flex gap-1">
          <button
            onClick={() => props.onEdit(props.project)}
            class="rounded p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-600 dark:hover:bg-gray-700 dark:hover:text-gray-300"
            title="Editar grupo"
          >
            ✏️
          </button>
          <button
            onClick={() => props.onDelete(props.project)}
            class="rounded p-1 text-gray-400 hover:bg-gray-100 hover:text-red-600 dark:hover:bg-gray-700"
            title="Eliminar grupo"
          >
            🗑️
          </button>
        </div>
      </div>

      {/* Descripción */}
      <p class="mb-2 line-clamp-3 flex-1 text-xs text-gray-600 dark:text-gray-400">
        {props.project.description}
      </p>

      {/* Badge de contador */}
      <div class="mb-2">
        <span
          class="inline-flex items-center gap-1 rounded-full px-2 py-1 text-xs font-medium"
          style={{
            'background-color': borderColor() + '20',
            color: borderColor(),
          }}
        >
          {hasSubprojects() ? (
            <>
              📊 {subprojectCount()}{' '}
              {subprojectCount() === 1 ? 'proyecto' : 'proyectos'}
            </>
          ) : (
            <>📂 Sin proyectos</>
          )}
        </span>
      </div>

      {/* Tags */}
      {props.project.notes && (
        <div class="mb-2 flex flex-wrap gap-1">
          <For each={props.project.notes.split(',').slice(0, 3)}>
            {(tag) => (
              <span class="rounded bg-gray-100 px-1.5 py-0.5 text-xs text-gray-600 dark:bg-gray-700 dark:text-gray-300">
                {tag.trim()}
              </span>
            )}
          </For>
        </div>
      )}

      {/* Estado del grupo */}
      {props.project.status && (
        <div class="mb-2">
          <span
            class={`inline-block rounded-full px-2 py-0.5 text-xs font-medium ${
              props.project.status === 'activo'
                ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                : props.project.status === 'pausado'
                  ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                  : props.project.status === 'completado'
                    ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
                    : 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200'
            }`}
          >
            {props.project.status.charAt(0).toUpperCase() +
              props.project.status.slice(1)}
          </span>
        </div>
      )}

      {/* Botón Ver Proyectos */}
      <button
        onClick={() => props.onViewProjects(props.project)}
        class="mt-auto w-full rounded-lg px-2 py-1.5 text-sm font-medium text-white transition-colors"
        style={{
          'background-color': borderColor(),
        }}
      >
        {hasSubprojects() ? '👁️ Ver Proyectos' : '➕ Agregar Proyectos'}
      </button>
    </div>
  );
};

export default GroupCard;
