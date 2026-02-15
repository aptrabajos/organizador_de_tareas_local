import { Component, Show } from 'solid-js';

interface SearchBarProps {
  onSearch: (query: string) => void;
  value?: string;
}

const SearchBar: Component<SearchBarProps> = (props) => {
  const handleInput = (
    e: InputEvent & { currentTarget: HTMLInputElement; target: Element }
  ) => {
    props.onSearch(e.currentTarget.value);
  };

  const handleClear = () => {
    props.onSearch('');
  };

  return (
    <div class="relative w-full">
      {/* Search Icon */}
      <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3.5">
        <svg
          class="h-4 w-4 text-surface-400 dark:text-surface-500"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
          />
        </svg>
      </div>

      {/* Input */}
      <input
        type="text"
        value={props.value || ''}
        onInput={handleInput}
        placeholder="Buscar proyectos..."
        class="input input-with-icon pr-10"
      />

      {/* Clear Button */}
      <Show when={props.value && props.value.length > 0}>
        <button
          onClick={handleClear}
          aria-label="Limpiar búsqueda"
          class="absolute inset-y-0 right-0 flex items-center pr-3 text-surface-400 transition-colors hover:text-surface-600 dark:text-surface-500 dark:hover:text-surface-300"
        >
          <svg
            class="h-4 w-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>
      </Show>

      {/* Keyboard shortcut hint */}
      <Show when={!props.value || props.value.length === 0}>
        <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-3">
          <kbd class="hidden rounded border border-surface-200 bg-surface-100 px-1.5 py-0.5 font-mono text-xs text-surface-400 dark:border-surface-700 dark:bg-surface-800 dark:text-surface-500 sm:inline-block">
            Ctrl+F
          </kbd>
        </div>
      </Show>
    </div>
  );
};

export default SearchBar;
