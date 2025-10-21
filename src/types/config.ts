// ==================== TIPOS DE CONFIGURACIÓN ====================

export type ProgramMode = 'auto' | 'default' | 'custom' | 'script';
export type ThemeMode = 'light' | 'dark' | 'auto';
export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';

// Configuración de un programa (terminal, navegador, etc.)
export interface ProgramConfig {
  mode: ProgramMode;
  custom_path?: string;
  custom_args: string[];
  custom_script?: string;
}

// Configuración de plataforma
export interface PlatformConfig {
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

// ==================== VALIDACIÓN ====================

export interface ValidationIssue {
  field: string;
  message: string;
}

export interface ValidationResult {
  is_valid: boolean;
  issues: ValidationIssue[];
}

// ==================== ESTADO DE CONFIGURACIÓN ====================

export interface ConfigState {
  config: AppConfig | null;
  isLoading: boolean;
  error: string | null;
  detectedPrograms: DetectedPrograms | null;
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
  | 'analytics'
  | 'refresh'
  | 'close_modal';

// Mapa de acciones a descripciones legibles
export const SHORTCUT_DESCRIPTIONS: Record<ShortcutAction, string> = {
  new_project: 'Crear nuevo proyecto',
  search: 'Buscar proyectos',
  settings: 'Abrir configuración',
  analytics: 'Abrir estadísticas',
  refresh: 'Recargar lista de proyectos',
  close_modal: 'Cerrar modal activo',
};
