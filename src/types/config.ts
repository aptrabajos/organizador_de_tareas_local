// ==================== TIPOS DE CONFIGURACIÓN ====================

export type ProgramMode = 'auto' | 'default' | 'custom' | 'script';
export type ThemeMode = 'light' | 'dark' | 'system';
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
  enabled: boolean;
  auto_backup: boolean;
  backup_interval_hours: number;
  backup_path?: string;
  max_backups: number;
}

// Configuración de UI
export interface UiConfig {
  theme: ThemeMode;
  language: string;
  show_analytics_on_startup: boolean;
  compact_mode: boolean;
}

// Configuración avanzada
export interface AdvancedConfig {
  log_level: LogLevel;
  enable_telemetry: boolean;
  check_updates_on_startup: boolean;
  custom_css?: string;
}

// Configuración completa de la aplicación
export interface AppConfig {
  version: string;
  platform: PlatformConfig;
  backup: BackupConfig;
  ui: UiConfig;
  advanced: AdvancedConfig;
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
