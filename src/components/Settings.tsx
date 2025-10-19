import { createSignal, Show, For, onMount } from 'solid-js';
import type {
  AppConfig,
  DetectedPrograms,
  ProgramMode,
  ProgramConfig,
} from '../types/config';
import {
  getConfig,
  updateConfig,
  resetConfig,
  detectPrograms,
  selectBackupFolder,
} from '../services/api';

type Tab = 'programs' | 'backup' | 'ui' | 'advanced';

export default function Settings(props: { onClose: () => void }) {
  const [activeTab, setActiveTab] = createSignal<Tab>('programs');
  const [config, setConfig] = createSignal<AppConfig | null>(null);
  const [detectedPrograms, setDetectedPrograms] =
    createSignal<DetectedPrograms | null>(null);
  const [isLoading, setIsLoading] = createSignal(false);
  const [isSaving, setIsSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [successMessage, setSuccessMessage] = createSignal<string | null>(null);

  // Cargar configuración y programas detectados
  onMount(async () => {
    await loadConfig();
    await loadDetectedPrograms();
  });

  const loadConfig = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const cfg = await getConfig();
      setConfig(cfg);
    } catch (err) {
      setError(`Error al cargar configuración: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  const loadDetectedPrograms = async () => {
    try {
      const programs = await detectPrograms();
      setDetectedPrograms(programs);
    } catch (err) {
      console.error('Error detectando programas:', err);
    }
  };

  const handleSave = async () => {
    const cfg = config();
    if (!cfg) return;

    setIsSaving(true);
    setError(null);
    setSuccessMessage(null);
    try {
      await updateConfig(cfg);
      setSuccessMessage('✅ Configuración guardada exitosamente');
      window.setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      setError(`Error al guardar: ${err}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleReset = async () => {
    if (
      !confirm(
        '¿Estás seguro de resetear la configuración a valores por defecto?'
      )
    )
      return;

    setIsLoading(true);
    setError(null);
    try {
      const cfg = await resetConfig();
      setConfig(cfg);
      setSuccessMessage('✅ Configuración reseteada');
      window.setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      setError(`Error al resetear: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  const updateProgramConfig = (
    type: 'terminal' | 'browser' | 'file_manager' | 'text_editor',
    updates: Partial<ProgramConfig>
  ) => {
    const cfg = config();
    if (!cfg) return;

    setConfig({
      ...cfg,
      platform: {
        ...cfg.platform,
        [type]: {
          ...cfg.platform[type],
          ...updates,
        },
      },
    });
  };

  const renderProgramConfig = (
    title: string,
    type: 'terminal' | 'browser' | 'file_manager' | 'text_editor',
    detectedList: any[]
  ) => {
    const cfg = config();
    if (!cfg) return null;

    const programCfg = cfg.platform[type];

    return (
      <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
          {title}
        </h3>

        {/* Modo de operación */}
        <div>
          <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
            Modo de Operación
          </label>
          <select
            class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
            value={programCfg.mode}
            onChange={(e) =>
              updateProgramConfig(type, {
                mode: e.currentTarget.value as ProgramMode,
              })
            }
          >
            <option value="auto">Auto - Detectar automáticamente</option>
            <option value="default">
              Default - Usar predeterminado del sistema
            </option>
            <option value="custom">Custom - Programa personalizado</option>
            <option value="script">
              Script - Ejecutar script personalizado
            </option>
          </select>
        </div>

        {/* Programas detectados (solo en modo auto) */}
        <Show when={programCfg.mode === 'auto' && detectedList.length > 0}>
          <div>
            <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
              Programas Detectados
            </label>
            <div class="space-y-2">
              <For each={detectedList}>
                {(program) => (
                  <div class="flex items-center justify-between rounded border border-gray-200 bg-white p-2 dark:border-gray-600 dark:bg-gray-700">
                    <div>
                      <span class="font-medium text-gray-900 dark:text-white">
                        {program.name}
                      </span>
                      <span class="ml-2 text-xs text-gray-500 dark:text-gray-400">
                        {program.path}
                      </span>
                    </div>
                    <Show when={program.is_default}>
                      <span class="rounded bg-blue-100 px-2 py-1 text-xs text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                        Por defecto
                      </span>
                    </Show>
                  </div>
                )}
              </For>
            </div>
          </div>
        </Show>

        {/* Configuración custom */}
        <Show when={programCfg.mode === 'custom'}>
          <div class="space-y-3">
            <div>
              <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
                Ruta del Programa
              </label>
              <input
                type="text"
                class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
                placeholder="/usr/bin/programa"
                value={programCfg.custom_path || ''}
                onInput={(e) =>
                  updateProgramConfig(type, {
                    custom_path: e.currentTarget.value,
                  })
                }
              />
            </div>
            <div>
              <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
                Argumentos (uno por línea)
              </label>
              <textarea
                class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 font-mono text-sm text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
                rows={3}
                placeholder="--arg1&#10;--arg2&#10;{path}"
                value={programCfg.custom_args.join('\n')}
                onInput={(e) =>
                  updateProgramConfig(type, {
                    custom_args: e.currentTarget.value
                      .split('\n')
                      .filter((s) => s.trim()),
                  })
                }
              />
              <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                Variables disponibles: {'{path}'}, {'{url}'}
              </p>
            </div>
          </div>
        </Show>

        {/* Configuración script */}
        <Show when={programCfg.mode === 'script'}>
          <div>
            <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
              Script Personalizado
            </label>
            <textarea
              class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 font-mono text-sm text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
              rows={5}
              placeholder="#!/bin/bash&#10;cd {path}&#10;alacritty &"
              value={programCfg.custom_script || ''}
              onInput={(e) =>
                updateProgramConfig(type, {
                  custom_script: e.currentTarget.value,
                })
              }
            />
            <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
              Variables disponibles: {'{path}'}, {'{url}'}
            </p>
          </div>
        </Show>
      </div>
    );
  };

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 p-4">
      <div class="flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden rounded-lg bg-white shadow-2xl dark:bg-gray-900">
        {/* Header */}
        <div class="flex items-center justify-between border-b border-gray-200 p-6 dark:border-gray-700">
          <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
            ⚙️ Configuración
          </h2>
          <button
            onClick={props.onClose}
            class="text-2xl text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
          >
            ×
          </button>
        </div>

        {/* Tabs */}
        <div class="flex border-b border-gray-200 px-6 dark:border-gray-700">
          <button
            class={`border-b-2 px-4 py-3 font-medium transition-colors ${
              activeTab() === 'programs'
                ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'
            }`}
            onClick={() => setActiveTab('programs')}
          >
            🖥️ Programas
          </button>
          <button
            class={`border-b-2 px-4 py-3 font-medium transition-colors ${
              activeTab() === 'backup'
                ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'
            }`}
            onClick={() => setActiveTab('backup')}
          >
            💾 Backups
          </button>
          <button
            class={`border-b-2 px-4 py-3 font-medium transition-colors ${
              activeTab() === 'ui'
                ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'
            }`}
            onClick={() => setActiveTab('ui')}
          >
            🎨 Interfaz
          </button>
          <button
            class={`border-b-2 px-4 py-3 font-medium transition-colors ${
              activeTab() === 'advanced'
                ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'
            }`}
            onClick={() => setActiveTab('advanced')}
          >
            🔧 Avanzado
          </button>
        </div>

        {/* Content */}
        <div class="flex-1 overflow-y-auto p-6">
          <Show when={isLoading()}>
            <div class="flex items-center justify-center py-12">
              <div class="h-12 w-12 animate-spin rounded-full border-b-2 border-blue-500" />
            </div>
          </Show>

          <Show when={error()}>
            <div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-4 dark:border-red-800 dark:bg-red-900/20">
              <p class="text-red-800 dark:text-red-200">{error()}</p>
            </div>
          </Show>

          <Show when={successMessage()}>
            <div class="mb-4 rounded-lg border border-green-200 bg-green-50 p-4 dark:border-green-800 dark:bg-green-900/20">
              <p class="text-green-800 dark:text-green-200">
                {successMessage()}
              </p>
            </div>
          </Show>

          <Show when={!isLoading() && config()}>
            {/* Tab: Programas */}
            <Show when={activeTab() === 'programs'}>
              <div class="space-y-6">
                <p class="mb-4 text-gray-600 dark:text-gray-400">
                  Configura qué programas usar para abrir terminales, enlaces,
                  archivos, etc.
                </p>
                {renderProgramConfig(
                  '🖥️ Terminal',
                  'terminal',
                  detectedPrograms()?.terminals || []
                )}
                {renderProgramConfig(
                  '🌐 Navegador',
                  'browser',
                  detectedPrograms()?.browsers || []
                )}
                {renderProgramConfig(
                  '📁 Gestor de Archivos',
                  'file_manager',
                  detectedPrograms()?.file_managers || []
                )}
                {renderProgramConfig(
                  '📝 Editor de Texto',
                  'text_editor',
                  detectedPrograms()?.text_editors || []
                )}
              </div>
            </Show>

            {/* Tab: Backups */}
            <Show when={activeTab() === 'backup'}>
              <div class="space-y-6">
                <p class="mb-4 text-gray-600 dark:text-gray-400">
                  Configuración de backups automáticos de proyectos.
                </p>

                {/* Backup automático habilitado */}
                <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <div class="flex items-center justify-between">
                    <div>
                      <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                        Backup Automático
                      </h3>
                      <p class="text-sm text-gray-600 dark:text-gray-400">
                        Crear backups automáticos de tus proyectos
                      </p>
                    </div>
                    <label class="relative inline-flex cursor-pointer items-center">
                      <input
                        type="checkbox"
                        class="peer sr-only"
                        checked={config()?.backup.auto_backup_enabled || false}
                        onChange={(e) => {
                          const cfg = config();
                          if (cfg) {
                            setConfig({
                              ...cfg,
                              backup: {
                                ...cfg.backup,
                                auto_backup_enabled: e.currentTarget.checked,
                              },
                            });
                          }
                        }}
                      />
                      <div class="peer h-6 w-11 rounded-full bg-gray-200 after:absolute after:start-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:border-gray-600 dark:bg-gray-700 dark:peer-focus:ring-blue-800 rtl:peer-checked:after:-translate-x-full" />
                    </label>
                  </div>

                  <Show when={config()?.backup.auto_backup_enabled}>
                    {/* Intervalo de backup */}
                    <div>
                      <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
                        Intervalo de Backup (días)
                      </label>
                      <input
                        type="number"
                        min="1"
                        max="30"
                        class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
                        value={config()?.backup.auto_backup_interval || 7}
                        onInput={(e) => {
                          const cfg = config();
                          if (cfg) {
                            setConfig({
                              ...cfg,
                              backup: {
                                ...cfg.backup,
                                auto_backup_interval: parseInt(
                                  e.currentTarget.value
                                ),
                              },
                            });
                          }
                        }}
                      />
                      <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                        Crear backup cada N días automáticamente
                      </p>
                    </div>
                  </Show>
                </div>

                {/* Limpieza de backups antiguos */}
                <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <div class="flex items-center justify-between">
                    <div>
                      <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                        Limpiar Backups Antiguos
                      </h3>
                      <p class="text-sm text-gray-600 dark:text-gray-400">
                        Eliminar backups más antiguos que cierto tiempo
                      </p>
                    </div>
                    <label class="relative inline-flex cursor-pointer items-center">
                      <input
                        type="checkbox"
                        class="peer sr-only"
                        checked={config()?.backup.cleanup_old_backups || false}
                        onChange={(e) => {
                          const cfg = config();
                          if (cfg) {
                            setConfig({
                              ...cfg,
                              backup: {
                                ...cfg.backup,
                                cleanup_old_backups: e.currentTarget.checked,
                              },
                            });
                          }
                        }}
                      />
                      <div class="peer h-6 w-11 rounded-full bg-gray-200 after:absolute after:start-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:border-gray-600 dark:bg-gray-700 dark:peer-focus:ring-blue-800 rtl:peer-checked:after:-translate-x-full" />
                    </label>
                  </div>

                  <Show when={config()?.backup.cleanup_old_backups}>
                    {/* Días de retención */}
                    <div>
                      <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
                        Días de Retención
                      </label>
                      <input
                        type="number"
                        min="7"
                        max="365"
                        class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
                        value={config()?.backup.retention_days || 30}
                        onInput={(e) => {
                          const cfg = config();
                          if (cfg) {
                            setConfig({
                              ...cfg,
                              backup: {
                                ...cfg.backup,
                                retention_days: parseInt(e.currentTarget.value),
                              },
                            });
                          }
                        }}
                      />
                      <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                        Eliminar backups más antiguos que N días
                      </p>
                    </div>
                  </Show>
                </div>
              </div>
            </Show>

            {/* Tab: UI */}
            <Show when={activeTab() === 'ui'}>
              <div class="space-y-6">
                <p class="mb-4 text-gray-600 dark:text-gray-400">
                  Personaliza la apariencia de la aplicación.
                </p>

                {/* Tema */}
                <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                    Tema
                  </h3>
                  <div>
                    <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
                      Modo de Color
                    </label>
                    <select
                      class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
                      value={config()?.ui.theme || 'auto'}
                      onChange={(e) => {
                        const cfg = config();
                        if (cfg) {
                          setConfig({
                            ...cfg,
                            ui: {
                              ...cfg.ui,
                              theme: e.currentTarget.value as
                                | 'light'
                                | 'dark'
                                | 'auto',
                            },
                          });
                        }
                      }}
                    >
                      <option value="light">☀️ Claro</option>
                      <option value="dark">🌙 Oscuro</option>
                      <option value="auto">🔄 Automático (sistema)</option>
                    </select>
                    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                      Modo automático sigue la configuración del sistema
                    </p>
                  </div>
                </div>

                {/* Idioma */}
                <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                    Idioma
                  </h3>
                  <div>
                    <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
                      Idioma de la Interfaz
                    </label>
                    <select
                      class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
                      value={config()?.ui.language || 'es'}
                      onChange={(e) => {
                        const cfg = config();
                        if (cfg) {
                          setConfig({
                            ...cfg,
                            ui: {
                              ...cfg.ui,
                              language: e.currentTarget.value,
                            },
                          });
                        }
                      }}
                    >
                      <option value="es">🇪🇸 Español</option>
                      <option value="en">🇺🇸 English</option>
                    </select>
                    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                      Requiere reiniciar la aplicación
                    </p>
                  </div>
                </div>

                {/* Opciones de confirmación */}
                <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                    Confirmaciones
                  </h3>
                  <div class="flex items-center justify-between">
                    <div>
                      <p class="font-medium text-gray-900 dark:text-white">
                        Confirmar antes de eliminar
                      </p>
                      <p class="text-sm text-gray-600 dark:text-gray-400">
                        Pedir confirmación al eliminar proyectos
                      </p>
                    </div>
                    <label class="relative inline-flex cursor-pointer items-center">
                      <input
                        type="checkbox"
                        class="peer sr-only"
                        checked={config()?.ui.confirm_delete ?? true}
                        onChange={(e) => {
                          const cfg = config();
                          if (cfg) {
                            setConfig({
                              ...cfg,
                              ui: {
                                ...cfg.ui,
                                confirm_delete: e.currentTarget.checked,
                              },
                            });
                          }
                        }}
                      />
                      <div class="peer h-6 w-11 rounded-full bg-gray-200 after:absolute after:start-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:border-gray-600 dark:bg-gray-700 dark:peer-focus:ring-blue-800 rtl:peer-checked:after:-translate-x-full" />
                    </label>
                  </div>

                  <div class="flex items-center justify-between">
                    <div>
                      <p class="font-medium text-gray-900 dark:text-white">
                        Mostrar bienvenida
                      </p>
                      <p class="text-sm text-gray-600 dark:text-gray-400">
                        Mostrar pantalla de bienvenida al iniciar
                      </p>
                    </div>
                    <label class="relative inline-flex cursor-pointer items-center">
                      <input
                        type="checkbox"
                        class="peer sr-only"
                        checked={config()?.ui.show_welcome ?? true}
                        onChange={(e) => {
                          const cfg = config();
                          if (cfg) {
                            setConfig({
                              ...cfg,
                              ui: {
                                ...cfg.ui,
                                show_welcome: e.currentTarget.checked,
                              },
                            });
                          }
                        }}
                      />
                      <div class="peer h-6 w-11 rounded-full bg-gray-200 after:absolute after:start-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:border-gray-600 dark:bg-gray-700 dark:peer-focus:ring-blue-800 rtl:peer-checked:after:-translate-x-full" />
                    </label>
                  </div>
                </div>
              </div>
            </Show>

            {/* Tab: Avanzado */}
            <Show when={activeTab() === 'advanced'}>
              <div class="space-y-6">
                <p class="mb-4 text-gray-600 dark:text-gray-400">
                  Configuración avanzada del sistema.
                </p>

                {/* Nivel de log */}
                <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                    Logs y Depuración
                  </h3>
                  <div>
                    <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
                      Nivel de Logs
                    </label>
                    <select
                      class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
                      value={config()?.advanced.log_level || 'info'}
                      onChange={(e) => {
                        const cfg = config();
                        if (cfg) {
                          setConfig({
                            ...cfg,
                            advanced: {
                              ...cfg.advanced,
                              log_level: e.currentTarget.value as
                                | 'trace'
                                | 'debug'
                                | 'info'
                                | 'warn'
                                | 'error',
                            },
                          });
                        }
                      }}
                    >
                      <option value="error">
                        Error - Solo errores críticos
                      </option>
                      <option value="warn">
                        Warn - Advertencias y errores
                      </option>
                      <option value="info">Info - Información general</option>
                      <option value="debug">Debug - Modo depuración</option>
                      <option value="trace">
                        Trace - Todo (muy detallado)
                      </option>
                    </select>
                    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                      Mayor nivel = más logs en consola
                    </p>
                  </div>
                </div>

                {/* Analytics */}
                <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                    Privacidad y Datos
                  </h3>
                  <div class="flex items-center justify-between">
                    <div>
                      <p class="font-medium text-gray-900 dark:text-white">
                        Habilitar Analytics
                      </p>
                      <p class="text-sm text-gray-600 dark:text-gray-400">
                        Enviar estadísticas anónimas de uso
                      </p>
                    </div>
                    <label class="relative inline-flex cursor-pointer items-center">
                      <input
                        type="checkbox"
                        class="peer sr-only"
                        checked={config()?.advanced.enable_analytics ?? true}
                        onChange={(e) => {
                          const cfg = config();
                          if (cfg) {
                            setConfig({
                              ...cfg,
                              advanced: {
                                ...cfg.advanced,
                                enable_analytics: e.currentTarget.checked,
                              },
                            });
                          }
                        }}
                      />
                      <div class="peer h-6 w-11 rounded-full bg-gray-200 after:absolute after:start-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:border-gray-600 dark:bg-gray-700 dark:peer-focus:ring-blue-800 rtl:peer-checked:after:-translate-x-full" />
                    </label>
                  </div>
                </div>

                {/* Auto-updates */}
                <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                    Actualizaciones
                  </h3>
                  <div class="flex items-center justify-between">
                    <div>
                      <p class="font-medium text-gray-900 dark:text-white">
                        Actualizaciones Automáticas
                      </p>
                      <p class="text-sm text-gray-600 dark:text-gray-400">
                        Buscar y descargar actualizaciones al iniciar
                      </p>
                    </div>
                    <label class="relative inline-flex cursor-pointer items-center">
                      <input
                        type="checkbox"
                        class="peer sr-only"
                        checked={config()?.advanced.enable_auto_update ?? true}
                        onChange={(e) => {
                          const cfg = config();
                          if (cfg) {
                            setConfig({
                              ...cfg,
                              advanced: {
                                ...cfg.advanced,
                                enable_auto_update: e.currentTarget.checked,
                              },
                            });
                          }
                        }}
                      />
                      <div class="peer h-6 w-11 rounded-full bg-gray-200 after:absolute after:start-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:border-gray-600 dark:bg-gray-700 dark:peer-focus:ring-blue-800 rtl:peer-checked:after:-translate-x-full" />
                    </label>
                  </div>
                  <p class="text-xs text-gray-500 dark:text-gray-400">
                    🚧 Nota: Auto-updates estará disponible en v0.3.0
                  </p>
                </div>
              </div>
            </Show>
          </Show>
        </div>

        {/* Footer */}
        <div class="flex items-center justify-between border-t border-gray-200 bg-gray-50 p-6 dark:border-gray-700 dark:bg-gray-800">
          <button
            onClick={handleReset}
            disabled={isLoading() || isSaving()}
            class="rounded-md px-4 py-2 text-red-600 transition-colors hover:bg-red-50 disabled:opacity-50 dark:text-red-400 dark:hover:bg-red-900/20"
          >
            🔄 Resetear
          </button>
          <div class="flex gap-3">
            <button
              onClick={props.onClose}
              class="rounded-md border border-gray-300 px-6 py-2 text-gray-700 transition-colors hover:bg-gray-100 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
            >
              Cancelar
            </button>
            <button
              onClick={handleSave}
              disabled={isSaving()}
              class="rounded-md bg-blue-500 px-6 py-2 text-white transition-colors hover:bg-blue-600 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {isSaving() ? 'Guardando...' : '💾 Guardar'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
