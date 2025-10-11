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
      <input
        type="text"
        value={props.value || ''}
        onInput={handleInput}
        placeholder="Buscar proyecto..."
        class="w-full rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 px-4 py-2 pr-10 focus:border-blue-500 dark:focus:border-blue-400 focus:outline-none focus:ring-2 focus:ring-blue-200 dark:focus:ring-blue-800"
      />
      <Show when={props.value && props.value.length > 0}>
        <button
          onClick={handleClear}
          aria-label="Limpiar búsqueda"
          class="absolute right-2 top-1/2 -translate-y-1/2 rounded-full p-1 text-gray-400 dark:text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-600 hover:text-gray-600 dark:hover:text-gray-300"
        >
          ✕
        </button>
      </Show>
    </div>
  );
};

export default SearchBar;
