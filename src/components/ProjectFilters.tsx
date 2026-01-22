import { Component, Show } from 'solid-js';
import type { Accessor } from 'solid-js';

export interface ProjectFiltersProps {
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
    <div class="flex flex-wrap items-center gap-3">
      {/* Status Filter */}
      <div class="flex items-center gap-2">
        <span class="text-xs font-medium text-surface-500 dark:text-surface-400">
          Estado
        </span>
        <div class="relative">
          <select
            value={props.statusFilter()}
            onInput={(e) => props.setStatusFilter(e.currentTarget.value)}
            class="appearance-none rounded-lg border border-surface-200 bg-white py-1.5 pl-3 pr-8 text-xs font-medium text-surface-700 transition-colors focus:border-accent-500 focus:outline-none focus:ring-2 focus:ring-accent-500/20 dark:border-surface-600 dark:bg-surface-800 dark:text-surface-200"
          >
            <option value="all">Todos</option>
            <option value="activo">Activo</option>
            <option value="pausado">Pausado</option>
            <option value="completado">Completado</option>
            <option value="archivado">Archivado</option>
          </select>
          <svg
            class="pointer-events-none absolute right-2 top-1/2 h-4 w-4 -translate-y-1/2 text-surface-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 9l-7 7-7-7"
            />
          </svg>
        </div>
      </div>

      {/* Divider */}
      <div class="h-5 w-px bg-surface-200 dark:bg-surface-700" />

      {/* Pinned Toggle */}
      <label class="flex cursor-pointer select-none items-center gap-2 rounded-lg px-2 py-1.5 transition-colors hover:bg-surface-100 dark:hover:bg-surface-800">
        <div class="relative">
          <input
            type="checkbox"
            checked={props.showPinnedOnly()}
            onChange={(e) => props.setShowPinnedOnly(e.currentTarget.checked)}
            class="peer sr-only"
          />
          <div class="h-5 w-9 rounded-full bg-surface-200 transition-colors peer-checked:bg-accent-500 peer-focus:ring-2 peer-focus:ring-accent-500/20 dark:bg-surface-700" />
          <div class="absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-white shadow-sm transition-transform peer-checked:translate-x-4" />
        </div>
        <span class="text-xs font-medium text-surface-600 dark:text-surface-300">
          Solo favoritos
        </span>
      </label>

      {/* Clear Filters */}
      <Show when={props.statusFilter() !== 'all' || props.showPinnedOnly()}>
        <button
          onClick={() => {
            props.setStatusFilter('all');
            props.setShowPinnedOnly(false);
          }}
          class="flex items-center gap-1.5 rounded-lg border border-surface-200 bg-white px-2.5 py-1.5 text-xs font-medium text-surface-600 transition-colors hover:bg-surface-50 hover:text-surface-800 dark:border-surface-600 dark:bg-surface-800 dark:text-surface-300 dark:hover:bg-surface-700"
        >
          <svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
          Limpiar
        </button>
      </Show>

      {/* Counter */}
      <div class="ml-auto flex items-center gap-1.5">
        <span class="badge badge-surface">
          {props.filteredCount}
          <span class="mx-1 text-surface-400">/</span>
          {props.totalCount}
        </span>
        <span class="text-xs text-surface-500 dark:text-surface-400">
          proyectos
        </span>
      </div>
    </div>
  );
};

export default ProjectFilters;
