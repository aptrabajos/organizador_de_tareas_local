import { createSignal, createEffect, Show, For, onMount } from 'solid-js';
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
      setTimeout(() => setSuccessMessage(null), 3000);
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
      setTimeout(() => setSuccessMessage(null), 3000);
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
      <div class="space-y-4 p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
          {title}
        </h3>

        {/* Modo de operación */}
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Modo de Operación
          </label>
          <select
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            value={programCfg.mode}
            onChange={(e) =>
              updateProgramConfig(type, {
                mode: e.currentTarget.value as ProgramMode,
              })
            }
          >
            <option value="auto">Auto - Detectar automáticamente</option>
            <option value="default">Default - Usar predeterminado del sistema</option>
            <option value="custom">Custom - Programa personalizado</option>
            <option value="script">Script - Ejecutar script personalizado</option>
          </select>
        </div>

        {/* Programas detectados (solo en modo auto) */}
        <Show when={programCfg.mode === 'auto' && detectedList.length > 0}>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Programas Detectados
            </label>
            <div class="space-y-2">
              <For each={detectedList}>
                {(program) => (
                  <div class="flex items-center justify-between p-2 bg-white dark:bg-gray-700 rounded border border-gray-200 dark:border-gray-600">
                    <div>
                      <span class="font-medium text-gray-900 dark:text-white">
                        {program.name}
                      </span>
                      <span class="text-xs text-gray-500 dark:text-gray-400 ml-2">
                        {program.path}
                      </span>
                    </div>
                    <Show when={program.is_default}>
                      <span class="text-xs bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200 px-2 py-1 rounded">
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
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Ruta del Programa
              </label>
              <input
                type="text"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
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
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Argumentos (uno por línea)
              </label>
              <textarea
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
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
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                Variables disponibles: {'{path}'}, {'{url}'}
              </p>
            </div>
          </div>
        </Show>

        {/* Configuración script */}
        <Show when={programCfg.mode === 'script'}>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Script Personalizado
            </label>
            <textarea
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
              rows={5}
              placeholder="#!/bin/bash&#10;cd {path}&#10;alacritty &"
              value={programCfg.custom_script || ''}
              onInput={(e) =>
                updateProgramConfig(type, {
                  custom_script: e.currentTarget.value,
                })
              }
            />
            <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Variables disponibles: {'{path}'}, {'{url}'}
            </p>
          </div>
        </Show>
      </div>
    );
  };

  return (
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div class="bg-white dark:bg-gray-900 rounded-lg shadow-2xl max-w-4xl w-full max-h-[90vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
          <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
            ⚙️ Configuración
          </h2>
          <button
            onClick={props.onClose}
            class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 text-2xl"
          >
            ×
          </button>
        </div>

        {/* Tabs */}
        <div class="flex border-b border-gray-200 dark:border-gray-700 px-6">
          <button
            class={`px-4 py-3 font-medium border-b-2 transition-colors ${
              activeTab() === 'programs'
                ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'
            }`}
            onClick={() => setActiveTab('programs')}
          >
            🖥️ Programas
          </button>
          <button
            class={`px-4 py-3 font-medium border-b-2 transition-colors ${
              activeTab() === 'backup'
                ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'
            }`}
            onClick={() => setActiveTab('backup')}
          >
            💾 Backups
          </button>
          <button
            class={`px-4 py-3 font-medium border-b-2 transition-colors ${
              activeTab() === 'ui'
                ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'
            }`}
            onClick={() => setActiveTab('ui')}
          >
            🎨 Interfaz
          </button>
          <button
            class={`px-4 py-3 font-medium border-b-2 transition-colors ${
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
            <div class="flex justify-center items-center py-12">
              <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" />
            </div>
          </Show>

          <Show when={error()}>
            <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 mb-4">
              <p class="text-red-800 dark:text-red-200">{error()}</p>
            </div>
          </Show>

          <Show when={successMessage()}>
            <div class="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-4 mb-4">
              <p class="text-green-800 dark:text-green-200">
                {successMessage()}
              </p>
            </div>
          </Show>

          <Show when={!isLoading() && config()}>
            {/* Tab: Programas */}
            <Show when={activeTab() === 'programs'}>
              <div class="space-y-6">
                <p class="text-gray-600 dark:text-gray-400 mb-4">
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
                <p class="text-gray-600 dark:text-gray-400 mb-4">
                  Configuración de backups automáticos de proyectos.
                </p>

                {/* Backup automático habilitado */}
                <div class="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg space-y-4">
                  <div class="flex items-center justify-between">
                    <div>
                      <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                        Backup Automático
                      </h3>
                      <p class="text-sm text-gray-600 dark:text-gray-400">
                        Crear backups automáticos de tus proyectos
                      </p>
                    </div>
                    <label class="relative inline-flex items-center cursor-pointer">
                      <input
                        type="checkbox"
                        class="sr-only peer"
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
                      <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600" />
                    </label>
                  </div>

                  <Show when={config()?.backup.auto_backup_enabled}>
                    {/* Intervalo de backup */}
                    <div>
                      <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                        Intervalo de Backup (días)
                      </label>
                      <input
                        type="number"
                        min="1"
                        max="30"
                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
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
                      <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        Crear backup cada N días automáticamente
                      </p>
                    </div>
                  </Show>
                </div>

                {/* Limpieza de backups antiguos */}
                <div class="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg space-y-4">
                  <div class="flex items-center justify-between">
                    <div>
                      <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                        Limpiar Backups Antiguos
                      </h3>
                      <p class="text-sm text-gray-600 dark:text-gray-400">
                        Eliminar backups más antiguos que cierto tiempo
                      </p>
                    </div>
                    <label class="relative inline-flex items-center cursor-pointer">
                      <input
                        type="checkbox"
                        class="sr-only peer"
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
                      <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600" />
                    </label>
                  </div>

                  <Show when={config()?.backup.cleanup_old_backups}>
                    {/* Días de retención */}
                    <div>
                      <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                        Días de Retención
                      </label>
                      <input
                        type="number"
                        min="7"
                        max="365"
                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
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
                      <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        Eliminar backups más antiguos que N días
                      </p>
                    </div>
                  </Show>
                </div>
              </div>
            </Show>

            {/* Tab: UI */}
            <Show when={activeTab() === 'ui'}>
              <div class="space-y-4">
                <p class="text-gray-600 dark:text-gray-400 mb-4">
                  Personaliza la apariencia de la aplicación.
                </p>
                <div class="p-4 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg">
                  <p class="text-yellow-800 dark:text-yellow-200">
                    🚧 Configuración de interfaz - Próximamente
                  </p>
                </div>
              </div>
            </Show>

            {/* Tab: Avanzado */}
            <Show when={activeTab() === 'advanced'}>
              <div class="space-y-4">
                <p class="text-gray-600 dark:text-gray-400 mb-4">
                  Configuración avanzada del sistema.
                </p>
                <div class="p-4 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg">
                  <p class="text-yellow-800 dark:text-yellow-200">
                    🚧 Configuración avanzada - Próximamente
                  </p>
                </div>
              </div>
            </Show>
          </Show>
        </div>

        {/* Footer */}
        <div class="flex items-center justify-between p-6 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800">
          <button
            onClick={handleReset}
            disabled={isLoading() || isSaving()}
            class="px-4 py-2 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-md transition-colors disabled:opacity-50"
          >
            🔄 Resetear
          </button>
          <div class="flex gap-3">
            <button
              onClick={props.onClose}
              class="px-6 py-2 border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors text-gray-700 dark:text-gray-300"
            >
              Cancelar
            </button>
            <button
              onClick={handleSave}
              disabled={isSaving()}
              class="px-6 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSaving() ? 'Guardando...' : '💾 Guardar'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
