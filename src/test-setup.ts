import { afterEach, vi } from 'vitest';
import { cleanup } from '@solidjs/testing-library';
import packageJson from '../package.json';

// La versión del mock se lee de package.json a propósito: hardcodearla hizo que el
// setup quedara clavado en 0.4.3 mientras la app ya iba por otra versión.
const APP_VERSION = packageJson.version;

// ==================== window.matchMedia controlable ====================
// jsdom NO implementa matchMedia. Sin este mock, cualquier componente que
// consulte `prefers-color-scheme` explota. Además lo dejamos CONTROLABLE para
// poder testear el modo 'auto' del tema, que depende de que el sistema cambie
// de esquema con la app abierta.

const DARK_QUERY_FRAGMENT = 'prefers-color-scheme: dark';

type MediaChangeListener = (event: { matches: boolean; media: string }) => void;

const mediaListeners = new Set<MediaChangeListener>();
let systemPrefersDark = false;

/** Cambia el esquema del "sistema" y notifica a los listeners vivos. */
export function setSystemPrefersDark(value: boolean): void {
  systemPrefersDark = value;
  const event = { matches: value, media: `(${DARK_QUERY_FRAGMENT})` };
  mediaListeners.forEach((listener) => listener(event));
}

export function getSystemPrefersDark(): boolean {
  return systemPrefersDark;
}

window.matchMedia = vi.fn().mockImplementation((query: string) => ({
  matches: query.includes(DARK_QUERY_FRAGMENT) ? systemPrefersDark : false,
  media: query,
  onchange: null,
  addEventListener: (type: string, listener: MediaChangeListener) => {
    if (type === 'change') mediaListeners.add(listener);
  },
  removeEventListener: (type: string, listener: MediaChangeListener) => {
    if (type === 'change') mediaListeners.delete(listener);
  },
  // API vieja, por si algún componente la usa
  addListener: (listener: MediaChangeListener) => mediaListeners.add(listener),
  removeListener: (listener: MediaChangeListener) =>
    mediaListeners.delete(listener),
  dispatchEvent: () => false,
}));

// ==================== overrides de ui en get_config ====================
// El mock de `get_config` devuelve los defaults del backend. Un test que quiera
// probar, por ejemplo, `confirm_delete: false` o `theme: 'dark'` los pisa acá en
// vez de tener que remockear el invoke entero.

interface UiConfigOverrides {
  theme?: 'light' | 'dark' | 'auto';
  language?: string;
  confirm_delete?: boolean;
  show_welcome?: boolean;
}

let uiConfigOverrides: UiConfigOverrides = {};

export function setUiConfigOverrides(overrides: UiConfigOverrides): void {
  uiConfigOverrides = overrides;
}

// Mock de @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn((cmd: string) => {
    // Mocks para time tracking
    if (cmd === 'check_tracking_config') {
      return Promise.resolve(false);
    }
    if (cmd === 'get_time_stats') {
      return Promise.resolve({
        total_seconds: 0,
        session_count: 0,
        avg_session_seconds: 0,
        longest_session_seconds: 0,
        today_seconds: 0,
        week_seconds: 0,
      });
    }
    if (cmd === 'init_tracking') {
      return Promise.resolve('OK');
    }
    if (cmd === 'get_tracking_sessions') {
      return Promise.resolve([]);
    }
    if (cmd === 'get_tracking_status') {
      return Promise.resolve({
        is_tracking: false,
        elapsed_seconds: 0,
      });
    }
    // Mocks para git
    if (cmd === 'get_git_branch') {
      return Promise.resolve('main');
    }
    if (cmd === 'get_git_status') {
      return Promise.resolve('clean');
    }
    if (cmd === 'get_git_file_count') {
      // Debe respetar el contrato real `GitFileCount { modified, staged, untracked }`
      // (src/types/git.ts y el struct de Rust). Los tres valores son DISTINTOS a
      // propósito: si fueran iguales, un test que confunda campos pasaría igual.
      return Promise.resolve({ modified: 2, staged: 1, untracked: 3 });
    }
    if (cmd === 'get_git_ahead_behind') {
      return Promise.resolve([0, 0]);
    }
    if (cmd === 'get_recent_commits') {
      return Promise.resolve([]);
    }
    if (cmd === 'get_git_remote_url') {
      return Promise.resolve(null);
    }
    if (cmd === 'get_git_modified_files') {
      return Promise.resolve([]);
    }
    // Mocks para proyectos
    if (cmd === 'count_subprojects') {
      return Promise.resolve(0);
    }
    if (cmd === 'track_project_open') {
      return Promise.resolve();
    }
    if (cmd === 'start_work_session') {
      return Promise.resolve({
        session_id: 1,
        project_id: 1,
        project_name: 'Test Project',
        previous_session_stopped: false,
        tracking_initialized: false,
      });
    }
    if (cmd === 'stop_work_session') {
      return Promise.resolve(null);
    }
    if (cmd === 'get_work_session_status') {
      return Promise.resolve({
        is_tracking: false,
        project_id: null,
        project_path: null,
        elapsed_seconds: 0,
      });
    }
    if (cmd === 'get_root_projects') {
      return Promise.resolve([]);
    }
    if (cmd === 'get_subprojects') {
      return Promise.resolve([]);
    }
    if (cmd === 'assign_project_to_group') {
      return Promise.resolve();
    }
    // Mocks para config
    if (cmd === 'get_config') {
      return Promise.resolve({
        version: APP_VERSION,
        platform: {
          os_override: 'auto',
          terminal: { mode: 'auto', custom_args: [] },
          browser: { mode: 'auto', custom_args: [] },
          file_manager: { mode: 'auto', custom_args: [] },
          text_editor: { mode: 'auto', custom_args: [] },
          environment: {},
        },
        backup: {
          auto_backup_enabled: false,
          auto_backup_interval: 24,
          cleanup_old_backups: false,
          retention_days: 30,
        },
        ui: {
          theme: 'auto',
          language: 'es',
          confirm_delete: true,
          show_welcome: true,
          ...uiConfigOverrides,
        },
        advanced: {
          log_level: 'info',
          enable_analytics: true,
          enable_auto_update: true,
        },
        shortcuts: { enabled: true, shortcuts: {} },
      });
    }
    if (cmd === 'get_shortcuts_config') {
      return Promise.resolve({ enabled: true, shortcuts: {} });
    }
    // Mocks para dashboard
    if (cmd === 'get_dashboard_data') {
      return Promise.resolve({
        recent_projects: [],
        pending_todos: [],
        recent_journal_entries: [],
      });
    }
    // Mocks para journal
    if (cmd === 'get_journal_entries') {
      return Promise.resolve([]);
    }
    if (cmd === 'create_journal_entry') {
      return Promise.resolve({
        id: 1,
        project_id: 1,
        content: '',
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
      });
    }
    // Mocks para links
    if (cmd === 'get_project_links') {
      return Promise.resolve([]);
    }
    if (cmd === 'create_project_link') {
      return Promise.resolve({
        id: 1,
        project_id: 1,
        link_type: 'repository',
        title: '',
        url: '',
        created_at: '2025-01-01T00:00:00Z',
      });
    }
    // Mocks para attachments
    if (cmd === 'get_attachments') {
      return Promise.resolve([]);
    }
    // Default
    return Promise.resolve(undefined);
  }),
}));

// Mock de @tauri-apps/api/app
vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn(() => Promise.resolve(APP_VERSION)),
  getName: vi.fn(() => Promise.resolve('Gestor de Proyectos')),
  getTauriVersion: vi.fn(() => Promise.resolve('2.1.0')),
}));

// Mock de @tauri-apps/plugin-dialog
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(() => Promise.resolve(null)),
  save: vi.fn(() => Promise.resolve(null)),
}));

// Mock de @tauri-apps/plugin-fs
vi.mock('@tauri-apps/plugin-fs', () => ({
  writeTextFile: vi.fn(() => Promise.resolve()),
  readTextFile: vi.fn(() => Promise.resolve('')),
}));

// Mock de @tauri-apps/plugin-global-shortcut
vi.mock('@tauri-apps/plugin-global-shortcut', () => ({
  register: vi.fn(() => Promise.resolve()),
  unregisterAll: vi.fn(() => Promise.resolve()),
}));

// Mock de solid-toast
vi.mock('solid-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
    loading: vi.fn(),
    dismiss: vi.fn(),
    custom: vi.fn(),
  },
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    loading: vi.fn(),
    dismiss: vi.fn(),
    custom: vi.fn(),
  },
}));

// Mock de marked
vi.mock('marked', () => ({
  marked: {
    parse: vi.fn((text: string) => `<p>${text}</p>`),
    setOptions: vi.fn(),
    use: vi.fn(),
  },
}));

// Mock de dompurify
vi.mock('dompurify', () => ({
  default: {
    sanitize: vi.fn((html: string) => html),
  },
}));

// Cleanup después de cada test
afterEach(() => {
  cleanup();
  // El estado global del mock NO puede filtrarse entre tests: un test que deja
  // el sistema en oscuro o confirm_delete en false haría fallar (o peor, pasar)
  // al siguiente por motivos invisibles.
  systemPrefersDark = false;
  mediaListeners.clear();
  uiConfigOverrides = {};
  document.documentElement.classList.remove('dark');
  // jsdom no siempre expone `localStorage` como global (depende del origin), y
  // el ThemeContext también lo accede defensivamente por el mismo motivo.
  try {
    window.localStorage?.clear();
  } catch {
    // sin localStorage no hay nada que limpiar
  }
});
