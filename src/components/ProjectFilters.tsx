import { Component, Show } from 'solid-js';
import type { Accessor } from 'solid-js';

interface ProjectFiltersProps {
  statusFilter: Accessor<string>;
  setStatusFilter: (value: string) => void;
  showPinnedOnly: Accessor<boolean>;
  setShowPinnedOnly: (value: boolean) => void;
  filteredCount: number;
  totalCount: number;
  compact?: boolean; // Modo compacto para el header
}

const ProjectFilters: Component<ProjectFiltersProps> = (props) => {
  return (
    <div class="flex flex-wrap items-center gap-2">
      <div class="flex items-center gap-2">
        <span class="text-xs font-medium text-gray-700 dark:text-gray-300">
          Estado:
        </span>
        <select
          value={props.statusFilter()}
          onInput={(e) => props.setStatusFilter(e.currentTarget.value)}
          class="rounded-md border border-gray-300 bg-white px-2 py-1 text-xs text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
        >
          <option value="all">Todos</option>
          <option value="activo">Activo</option>
          <option value="pausado">Pausado</option>
          <option value="completado">Completado</option>
          <option value="archivado">Archivado</option>
        </select>
      </div>

      <label class="flex cursor-pointer items-center gap-1.5">
        <input
          type="checkbox"
          checked={props.showPinnedOnly()}
          onChange={(e) => props.setShowPinnedOnly(e.currentTarget.checked)}
          class="h-3.5 w-3.5 rounded border-gray-300 text-blue-600 focus:ring-2 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-700"
        />
        <span class="text-xs font-medium text-gray-700 dark:text-gray-300">
          📌 Solo favoritos
        </span>
      </label>

      <Show when={props.statusFilter() !== 'all' || props.showPinnedOnly()}>
        <button
          onClick={() => {
            props.setStatusFilter('all');
            props.setShowPinnedOnly(false);
          }}
          class="rounded-md bg-gray-100 px-2 py-1 text-xs font-medium text-gray-700 hover:bg-gray-200 dark:bg-gray-700 dark:text-gray-300 dark:hover:bg-gray-600"
        >
          Limpiar
        </button>
      </Show>

      <div class="ml-auto text-xs text-gray-600 dark:text-gray-400">
        {props.filteredCount} de {props.totalCount} proyectos
      </div>
    </div>
  );
};

export default ProjectFilters;
