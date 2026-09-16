import { Component, createSignal, onMount, For, Show } from 'solid-js';
import { createDroppable } from '@thisbeyond/solid-dnd';
import type { Project } from '../types/project';
import { countSubprojects, getSubprojects } from '../services/api';
import { logger } from '../utils/logger';

interface GroupCardProps {
  project: Project;
  onViewProjects: (project: Project) => void;
  onEdit: (project: Project) => void;
  onDelete: (project: Project) => void;
  onDropProject?: (projectId: number, groupId: number) => void; // v0.4.0 - Drag & Drop
}

// Constantes para colores y estilos
const DEFAULT_GROUP_COLOR = '#3B82F6';
const COLOR_OPACITY = '20'; // Para backgrounds translúcidos

// Utilidad para obtener clases de estado
const getStatusClasses = (status?: string): string => {
  switch (status) {
    case 'activo':
      return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
    case 'pausado':
      return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
    case 'completado':
      return 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200';
    default:
      return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200';
  }
};

// Utilidad para capitalizar primera letra
const capitalize = (str: string): string =>
  str.charAt(0).toUpperCase() + str.slice(1);

const GroupCard: Component<GroupCardProps> = (props) => {
  // Capturar valores inmutables de props (id no cambia durante el lifecycle)
  const projectId = props.project.id;

  const [subprojectCount, setSubprojectCount] = createSignal(0);
  const [showSubprojects, setShowSubprojects] = createSignal(false);
  const [subprojects, setSubprojects] = createSignal<Project[]>([]);
  const [loadingSubprojects, setLoadingSubprojects] = createSignal(false);

  // v0.4.0 - Hacer el GroupCard droppable para drag & drop
  const droppable = createDroppable(projectId);

  // Computed values para lógica reutilizable
  const borderColor = () => props.project.group_color || DEFAULT_GROUP_COLOR;
  const hasSubprojects = () => subprojectCount() > 0;
  const badgeBackgroundColor = () => `${borderColor()}${COLOR_OPACITY}`;

  onMount(async () => {
    try {
      const count = await countSubprojects(props.project.id);
      setSubprojectCount(count);
    } catch (err) {
      logger.error('Error counting subprojects:', err);
    }
  });

  // Cargar subproyectos cuando se expande la lista
  const toggleSubprojects = async () => {
    // Solo cargar si es la primera vez que expandimos
    if (!showSubprojects() && subprojects().length === 0) {
      setLoadingSubprojects(true);
      try {
        const projects = await getSubprojects(props.project.id);
        setSubprojects(projects);
      } catch (err) {
        logger.error('Error loading subprojects:', err);
      } finally {
        setLoadingSubprojects(false);
      }
    }
    setShowSubprojects(!showSubprojects());
  };

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
      {/* Header con imagen/ícono del grupo y título */}
      <div class="mb-2 flex items-start justify-between">
        <div class="flex items-center gap-2">
          {/* Mostrar imagen si existe, sino mostrar ícono del grupo */}
          <Show
            when={props.project.image_data}
            fallback={
              <span class="text-3xl">{props.project.group_icon || '📁'}</span>
            }
          >
            <img
              src={props.project.image_data}
              alt={props.project.name}
              class="h-12 w-12 flex-shrink-0 rounded-lg border-2 object-cover"
              style={{ 'border-color': borderColor() }}
            />
          </Show>
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

      {/* Badge de contador clickeable para expandir/colapsar */}
      <div class="mb-2">
        <Show
          when={hasSubprojects()}
          fallback={
            <span
              class="inline-flex items-center gap-1 rounded-full px-2 py-1 text-xs font-medium"
              style={{
                'background-color': badgeBackgroundColor(),
                color: borderColor(),
              }}
            >
              📂 Sin proyectos
            </span>
          }
        >
          <button
            onClick={toggleSubprojects}
            class="inline-flex items-center gap-1 rounded-full px-2 py-1 text-xs font-medium transition-all hover:brightness-90"
            style={{
              'background-color': badgeBackgroundColor(),
              color: borderColor(),
            }}
            title={
              showSubprojects()
                ? 'Ocultar subproyectos'
                : 'Mostrar subproyectos'
            }
          >
            {showSubprojects() ? '▼' : '▶'} 📊 {subprojectCount()}{' '}
            {subprojectCount() === 1 ? 'proyecto' : 'proyectos'}
          </button>
        </Show>
      </div>

      {/* Lista expandible de subproyectos */}
      <Show when={showSubprojects()}>
        <div class="mb-2 max-h-32 overflow-y-auto rounded border border-gray-200 bg-gray-50 p-2 dark:border-gray-600 dark:bg-gray-700">
          <Show
            when={!loadingSubprojects()}
            fallback={
              <p class="text-center text-xs text-gray-500 dark:text-gray-400">
                Cargando...
              </p>
            }
          >
            <ul class="space-y-1">
              <For each={subprojects()}>
                {(subproject) => (
                  <li class="flex items-center gap-2 text-xs">
                    <span class="text-gray-400">•</span>
                    <span class="truncate text-gray-700 dark:text-gray-300">
                      {subproject.name}
                    </span>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </div>
      </Show>

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
            class={`inline-block rounded-full px-2 py-0.5 text-xs font-medium ${getStatusClasses(props.project.status)}`}
          >
            {capitalize(props.project.status)}
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
