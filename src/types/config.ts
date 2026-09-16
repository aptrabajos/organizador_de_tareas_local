// ==================== TIPOS DE CONFIGURACIÓN ====================

export type ProgramMode = 'auto' | 'default' | 'custom' | 'script';
export type ThemeMode = 'light' | 'dark' | 'auto';
export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';
export type OsOverride = 'auto' | 'linux' | 'windows';

// Configuración de un programa (terminal, navegador, etc.)
export interface ProgramConfig {
  mode: ProgramMode;
  custom_path?: string;
  custom_args: string[];
  custom_script?: string;
}

// Configuración de plataforma
export interface PlatformConfig {
  os_override: OsOverride;
  terminal: ProgramConfig;
  browser: ProgramConfig;
  file_manager: ProgramConfig;
  text_editor: ProgramConfig;
  environment: Record<string, string>;
}

// Configuración de backups
export interface BackupConfig {
  default_path?: string;
  auto_backup_enabled: boolean;
  auto_backup_interval: number;
  cleanup_old_backups: boolean;
  retention_days: number;
  last_backup?: string;
}

// Resultado de un backup recién creado (matchea backup::BackupResult en Rust)
export interface BackupResult {
  file_path: string;
  size_bytes: number;
  created_at: string;
  integrity_ok: boolean;
  project_count: number;
}

// Entrada de la lista de backups existentes (matchea backup::BackupEntry en Rust)
export interface BackupEntry {
  file_path: string;
  filename: string;
  size_bytes: number;
  created_at: string;
  integrity_ok: boolean;
}

// Resultado de una restauración (matchea backup::RestoreResult en Rust)
export interface RestoreResult {
  restored_from: string;
  project_count: number;
}

// Configuración de UI
export interface UiConfig {
  theme: ThemeMode;
  language: string;
  confirm_delete: boolean;
  show_welcome: boolean;
}

// Configuración avanzada
export interface AdvancedConfig {
  log_level: LogLevel;
  enable_analytics: boolean;
  database_path?: string;
  enable_auto_update: boolean;
}

// Configuración completa de la aplicación
export interface AppConfig {
  version: string;
  platform: PlatformConfig;
  backup: BackupConfig;
  ui: UiConfig;
  advanced: AdvancedConfig;
  shortcuts: ShortcutsConfig;
}

// ==================== DETECCIÓN DE PROGRAMAS ====================

// Programa detectado en el sistema
export interface DetectedProgram {
  name: string;
  path: string;
  version?: string;
  is_default: boolean;
}

// Todos los programas detectados
export interface DetectedPrograms {
  terminals: DetectedProgram[];
  browsers: DetectedProgram[];
  file_managers: DetectedProgram[];
  text_editors: DetectedProgram[];
}

// ==================== ATAJOS DE TECLADO ====================

// Configuración de atajos de teclado individual
export interface ShortcutBinding {
  key: string; // ej: "Ctrl+N", "Cmd+T"
  enabled: boolean;
  description?: string;
}

// Configuración completa de atajos
export interface ShortcutsConfig {
  enabled: boolean;
  shortcuts: Record<string, ShortcutBinding>;
}

// Acciones disponibles para atajos
export type ShortcutAction =
  | 'new_project'
  | 'search'
  | 'settings'
  | 'about'
  | 'refresh'
  | 'close_modal';

// Metadata de cada atajo para la pantalla de configuración.
//
// Reemplaza al viejo `SHORTCUT_DESCRIPTIONS`, que mapeaba una sola string por
// acción y no tenía NINGÚN consumidor: Settings renderizaba seis bloques copiados
// con estos textos hardcodeados. Cada bloque necesita TRES datos (título,
// descripción y tecla por defecto), así que un `Record<ShortcutAction, string>`
// no alcanzaba para alimentarlos sin perder texto.
//
// El ORDEN de las claves es el orden en que se listan en la UI: `Object.entries`
// preserva el orden de inserción de claves string, así que no hay que duplicarlo.
export interface ShortcutMetadata {
  title: string;
  description: string;
  defaultKey: string;
}

export const SHORTCUT_METADATA: Record<ShortcutAction, ShortcutMetadata> = {
  new_project: {
    title: 'Nuevo Proyecto',
    description: 'Abrir formulario de nuevo proyecto',
    defaultKey: 'Ctrl+N',
  },
  search: {
    title: 'Buscar Proyectos',
    description: 'Focus en barra de búsqueda',
    defaultKey: 'Ctrl+F',
  },
  settings: {
    title: 'Abrir Configuración',
    description: 'Abrir este panel de configuración',
    defaultKey: 'Ctrl+Comma',
  },
  about: {
    title: 'Acerca de',
    description: 'Ver información de la aplicación',
    defaultKey: 'Ctrl+Shift+A',
  },
  refresh: {
    title: 'Recargar Proyectos',
    description: 'Actualizar lista de proyectos',
    defaultKey: 'Ctrl+R',
  },
  close_modal: {
    title: 'Cerrar Modal',
    description: 'Cerrar cualquier modal activo',
    defaultKey: 'Escape',
  },
};
