import { For } from 'solid-js';
import { useTheme, type ThemeMode } from '../contexts/ThemeContext';

// Con tres modos un toggle binario no alcanza: ciclar a ciegas obliga al usuario
// a adivinar en cuál está parado. Un selector segmentado muestra el modo activo.
const OPTIONS: { mode: ThemeMode; label: string; icon: string; title: string }[] =
  [
    { mode: 'light', label: 'Claro', icon: '☀️', title: 'Tema claro' },
    { mode: 'dark', label: 'Oscuro', icon: '🌙', title: 'Tema oscuro' },
    {
      mode: 'auto',
      label: 'Auto',
      icon: '🔄',
      title: 'Seguir la configuración del sistema',
    },
  ];

export default function ThemeToggle() {
  const { themeMode, setThemeMode } = useTheme();

  return (
    <div
      role="group"
      aria-label="Modo de color"
      class="fixed right-4 top-20 z-50 flex items-center gap-1 rounded-full border border-gray-200 bg-white p-1 shadow-lg dark:border-gray-700 dark:bg-gray-800"
    >
      <For each={OPTIONS}>
        {(option) => (
          <button
            onClick={() => setThemeMode(option.mode)}
            aria-label={option.title}
            aria-pressed={themeMode() === option.mode}
            title={option.title}
            class="flex items-center gap-1 rounded-full px-3 py-1.5 text-xs font-medium transition-colors"
            classList={{
              'bg-blue-600 text-white': themeMode() === option.mode,
              'text-gray-600 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-700':
                themeMode() !== option.mode,
            }}
          >
            <span aria-hidden="true">{option.icon}</span>
            <span>{option.label}</span>
          </button>
        )}
      </For>
    </div>
  );
}
