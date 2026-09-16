import { createSignal, Show, For, onMount } from 'solid-js';
import type {
  AppConfig,
  DetectedPrograms,
  DetectedProgram,
  ProgramMode,
  ProgramConfig,
  BackupResult,
  BackupEntry,
} from '../types/config';
import {
  getConfig,
  updateConfig,
  resetConfig,
  detectPrograms,
  selectBackupFolder,
  backupDatabase,
  listBackups,
  restoreBackup,
} from '../services/api';
import { getErrorMessage } from '../utils/errors';
import { useTheme } from '../contexts/ThemeContext';
import { SHORTCUT_METADATA, type ThemeMode } from '../types/config';
import { logger } from '../utils/logger';

type Tab = 'programs' | 'backup' | 'ui' | 'shortcuts' | 'advanced';

export default function Settings(props: { onClose: () => void }) {
  const { setThemeMode } = useTheme();
  const [activeTab, setActiveTab] = createSignal<Tab>('programs');
  const [config, setConfig] = createSignal<AppConfig | null>(null);
  const [detectedPrograms, setDetectedPrograms] =
    createSignal<DetectedPrograms | null>(null);
  const [isLoading, setIsLoading] = createSignal(false);
  const [isSaving, setIsSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [successMessage, setSuccessMessage] = createSignal<string | null>(null);

  // Estado del backup manual de la base de datos
  const [isBackingUp, setIsBackingUp] = createSignal(false);
  const [lastBackupResult, setLastBackupResult] =
    createSignal<BackupResult | null>(null);
  const [backupList, setBackupList] = createSignal<BackupEntry[]>([]);
  // Path del backup que se está restaurando (null = ninguno en curso). Se usa el
  // path como id porque deshabilita SOLO el botón clickeado, no toda la lista.
  const [restoringPath, setRestoringPath] = createSignal<string | null>(null);

  // Cargar configuración y programas detectados
  onMount(async () => {
    await loadConfig();
    await loadDetectedPrograms();
    await loadBackupList();
  });

  // Formatea bytes a un tamaño legible (KB/MB)
  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  };

  const loadBackupList = async () => {
    try {
      const list = await listBackups();
      setBackupList(list);
    } catch (err) {
      // No bloquea la pantalla: solo se loguea
      logger.error('Error al listar backups:', getErrorMessage(err));
    }
  };

  const handleBackupNow = async () => {
    setIsBackingUp(true);
    setError(null);
    setSuccessMessage(null);
    setLastBackupResult(null);
    try {
      // Sincronizar SOLO la config de backup (carpeta/retención) al backend antes del
      // backup, SIN pisar cambios sin guardar de otros tabs: partimos de la config
      // PERSISTIDA y mergeamos únicamente `backup`. run_backup lee la persistida.
      const cfg = config();
      if (cfg) {
        const persisted = await getConfig();
        await updateConfig({ ...persisted, backup: cfg.backup });
      }
      const result = await backupDatabase();
      setLastBackupResult(result);
      setSuccessMessage('✅ Backup creado correctamente');
      window.setTimeout(() => setSuccessMessage(null), 3000);
      // Refrescar config (last_backup) y la lista de backups
      await loadConfig();
      await loadBackupList();
    } catch (err) {
      setError(`Error al crear el backup: ${getErrorMessage(err)}`);
    } finally {
      setIsBackingUp(false);
    }
  };

  // Restaura la DB viva desde un backup elegido. Operación IRREVERSIBLE: reemplaza
  // TODOS los proyectos actuales por los del backup. El backend verifica integridad
  // antes y después de copiar, y solo confirma si el swap fue seguro; si algo sale
  // mal, la DB actual queda intacta. Si sale bien, la app se cierra sola (la conexión
  // viva quedó apuntando al archivo anterior) y hay que volver a abrirla.
  const handleRestore = async (entry: BackupEntry) => {
    const confirmed = window.confirm(
      `⚠️ ESTO REEMPLAZA TU BASE DE DATOS ACTUAL\n\n` +
        `Vas a restaurar el backup "${entry.filename}" (${entry.created_at}).\n\n` +
        `TODOS los proyectos, links, notas y tareas actuales se van a REEMPLAZAR ` +
        `por los del backup. Esta acción NO se puede deshacer.\n\n` +
        `La app se va a cerrar automáticamente al terminar; volvé a abrirla para ` +
        `ver los datos restaurados.\n\n` +
        `¿Confirmás la restauración?`
    );
    if (!confirmed) return;

    setRestoringPath(entry.file_path);
    setError(null);
    setSuccessMessage(null);
    try {
      const result = await restoreBackup(entry.file_path);
      setSuccessMessage(
        `✅ Restauración completa (${result.project_count} proyectos). Cerrando la app...`
      );
      // No hace falta setRestoringPath(null) ni refrescar nada: el backend cierra
      // la app en breve (~800ms) para forzar una conexión nueva a la DB restaurada.
    } catch (err) {
      setError(`Error al restaurar el backup: ${getErrorMessage(err)}`);
      setRestoringPath(null);
    }
  };

  // Persiste un cambio de config de backup AL INSTANTE, sin pisar cambios sin guardar
  // de otros tabs (parte de la config persistida y mergea solo `backup`). Necesario
  // porque el auto-backup se evalúa al ARRANCAR leyendo la config del disco.
  const persistBackupConfig = async (newCfg: AppConfig) => {
    const prev = config(); // por si falla la persistencia, revertimos
    setConfig(newCfg); // optimista
    try {
      const persisted = await getConfig();
      await updateConfig({ ...persisted, backup: newCfg.backup });
    } catch (err) {
      // La UI DEBE reflejar el disco (el auto-backup lee la config de ahí al
      // arrancar): si no se pudo guardar, revertimos el signal para no mentir.
      setConfig(prev);
      setError(getErrorMessage(err));
    }
  };

  const loadConfig = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const cfg = await getConfig();
      setConfig(cfg);
    } catch (err) {
      setError(`Error al cargar configuración: ${getErrorMessage(err)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const loadDetectedPrograms = async () => {
    try {
      const programs = await detectPrograms();
      setDetectedPrograms(programs);
    } catch (err) {
      logger.error('Error detectando programas:', err);
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
      setError(`Error al guardar: ${getErrorMessage(err)}`);
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
      setError(`Error al resetear: ${getErrorMessage(err)}`);
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
    detectedList: DetectedProgram[]
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
            onClick={() => props.onClose()}
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
              activeTab() === 'shortcuts'
                ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'
            }`}
            onClick={() => setActiveTab('shortcuts')}
          >
            ⌨️ Atajos
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

                {/* Selector de Sistema Operativo */}
                <div class="rounded-lg bg-blue-50 p-4 dark:bg-blue-900/20">
                  <h3 class="mb-2 text-lg font-semibold text-gray-900 dark:text-white">
                    💻 Sistema Operativo
                  </h3>
                  <p class="mb-3 text-sm text-gray-600 dark:text-gray-400">
                    Selecciona el sistema operativo para detectar y configurar
                    las herramientas apropiadas
                  </p>
                  <div>
                    <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
                      Sistema Operativo
                    </label>
                    <select
                      class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
                      value={config()?.platform.os_override || 'auto'}
                      onChange={(e) => {
                        const cfg = config();
                        if (cfg) {
                          setConfig({
                            ...cfg,
                            platform: {
                              ...cfg.platform,
                              os_override: e.currentTarget.value as
                                | 'auto'
                                | 'linux'
                                | 'windows',
                            },
                          });
                        }
                      }}
                    >
                      <option value="auto">
                        🔄 Auto - Detectar automáticamente
                      </option>
                      <option value="linux">
                        🐧 Linux - Herramientas Linux
                      </option>
                      <option value="windows">
                        🪟 Windows - Herramientas Windows
                      </option>
                    </select>
                    <p class="mt-2 text-xs text-gray-500 dark:text-gray-400">
                      💡 Tip: Usa "Auto" para detección automática. Solo cambia
                      manualmente si necesitas forzar herramientas específicas
                      de un sistema operativo.
                    </p>
                  </div>
                </div>

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
                  Backup de la base de datos y configuración de backups.
                </p>

                {/* Backup manual de la base de datos */}
                <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <div>
                    <h3 class="mb-2 text-lg font-semibold text-gray-900 dark:text-white">
                      💾 Backup de la Base de Datos
                    </h3>
                    <p class="mb-3 text-sm text-gray-600 dark:text-gray-400">
                      Crea una copia consistente y verificada de tu base de
                      datos en la carpeta de backups.
                    </p>

                    <div class="flex flex-wrap items-center gap-3">
                      <button
                        type="button"
                        disabled={isBackingUp()}
                        class="inline-flex items-center gap-2 rounded-md bg-green-600 px-4 py-2 text-white hover:bg-green-700 disabled:cursor-not-allowed disabled:opacity-60 dark:bg-green-500 dark:hover:bg-green-600"
                        onClick={handleBackupNow}
                      >
                        <Show
                          when={isBackingUp()}
                          fallback={<span>💾 Backup ahora</span>}
                        >
                          <span class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent" />
                          <span>Creando backup...</span>
                        </Show>
                      </button>

                      <Show when={config()?.backup.last_backup}>
                        <span class="text-sm text-gray-600 dark:text-gray-400">
                          Último backup:{' '}
                          <span class="font-medium text-gray-900 dark:text-white">
                            {config()?.backup.last_backup}
                          </span>
                        </span>
                      </Show>
                    </div>

                    {/* Resultado del último backup creado en esta sesión */}
                    <Show when={lastBackupResult()}>
                      {(result) => (
                        <div class="mt-4 rounded-md border border-green-200 bg-green-50 p-3 text-sm dark:border-green-800 dark:bg-green-900/20">
                          <p class="break-all text-gray-700 dark:text-gray-300">
                            <span class="font-medium">Archivo:</span>{' '}
                            {result().file_path}
                          </p>
                          <p class="text-gray-700 dark:text-gray-300">
                            <span class="font-medium">Tamaño:</span>{' '}
                            {formatBytes(result().size_bytes)}
                          </p>
                          <p class="text-gray-700 dark:text-gray-300">
                            <span class="font-medium">Integridad:</span>{' '}
                            <span
                              class={
                                result().integrity_ok
                                  ? 'font-semibold text-green-600 dark:text-green-400'
                                  : 'font-semibold text-red-600 dark:text-red-400'
                              }
                            >
                              {result().integrity_ok ? 'OK' : 'FALLÓ'}
                            </span>
                          </p>
                          <p class="text-gray-700 dark:text-gray-300">
                            <span class="font-medium">Proyectos:</span>{' '}
                            {result().project_count}
                          </p>
                        </div>
                      )}
                    </Show>
                  </div>

                  {/* Lista de backups existentes */}
                  <div>
                    <h4 class="mb-2 text-sm font-semibold text-gray-900 dark:text-white">
                      Backups disponibles ({backupList().length})
                    </h4>
                    <Show
                      when={backupList().length > 0}
                      fallback={
                        <p class="text-sm text-gray-500 dark:text-gray-400">
                          No hay backups todavía.
                        </p>
                      }
                    >
                      <ul class="max-h-64 space-y-1 overflow-y-auto">
                        <For each={backupList()}>
                          {(entry) => (
                            <li class="flex items-center justify-between gap-2 rounded-md bg-white px-3 py-2 text-sm dark:bg-gray-700">
                              <div class="min-w-0 flex-1">
                                <div class="flex items-center gap-1.5">
                                  <span
                                    class="truncate font-mono text-gray-700 dark:text-gray-300"
                                    title={entry.file_path}
                                  >
                                    {entry.filename}
                                  </span>
                                  <Show when={!entry.integrity_ok}>
                                    <span
                                      class="shrink-0 rounded bg-red-100 px-1.5 py-0.5 text-xs font-semibold text-red-700 dark:bg-red-900/30 dark:text-red-400"
                                      title="Este backup no pasó PRAGMA integrity_check y no se puede restaurar"
                                    >
                                      CORRUPTO
                                    </span>
                                  </Show>
                                </div>
                                <span class="text-xs text-gray-500 dark:text-gray-400">
                                  {formatBytes(entry.size_bytes)} ·{' '}
                                  {entry.created_at}
                                </span>
                              </div>
                              <button
                                type="button"
                                class="shrink-0 rounded-md border border-orange-300 px-2.5 py-1 text-xs font-medium text-orange-700 hover:bg-orange-50 disabled:cursor-not-allowed disabled:opacity-50 dark:border-orange-700 dark:text-orange-400 dark:hover:bg-orange-900/20"
                                disabled={
                                  !entry.integrity_ok ||
                                  restoringPath() === entry.file_path
                                }
                                title={
                                  entry.integrity_ok
                                    ? 'Restaurar este backup (reemplaza la DB actual)'
                                    : 'No se puede restaurar: falló la verificación de integridad'
                                }
                                onClick={() => handleRestore(entry)}
                              >
                                {restoringPath() === entry.file_path
                                  ? 'Restaurando...'
                                  : '♻️ Restaurar'}
                              </button>
                            </li>
                          )}
                        </For>
                      </ul>
                    </Show>
                  </div>
                </div>

                {/* Carpeta destino de backups */}
                <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <div>
                    <h3 class="mb-2 text-lg font-semibold text-gray-900 dark:text-white">
                      📁 Carpeta de Backups
                    </h3>
                    <p class="mb-3 text-sm text-gray-600 dark:text-gray-400">
                      Ubicación donde se guardarán los backups de tus proyectos
                    </p>
                    <div class="flex gap-2">
                      <input
                        type="text"
                        class="flex-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
                        value={
                          config()?.backup.default_path || '(No configurada)'
                        }
                        readonly
                        placeholder="Selecciona una carpeta..."
                      />
                      <button
                        type="button"
                        class="rounded-md bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600"
                        onClick={async () => {
                          try {
                            const selectedPath = await selectBackupFolder();
                            if (selectedPath) {
                              const cfg = config();
                              if (cfg) {
                                const newCfg = {
                                  ...cfg,
                                  backup: {
                                    ...cfg.backup,
                                    default_path: selectedPath,
                                  },
                                };
                                setConfig(newCfg);
                                // Persistir YA al backend: el backup lee la config del
                                // backend, NO este signal. Sin guardar acá, el backup iría
                                // a la carpeta vieja mientras la UI muestra la nueva.
                                await updateConfig(newCfg);
                                setSuccessMessage(
                                  `Carpeta de backup guardada: ${selectedPath}`
                                );
                                window.setTimeout(
                                  () => setSuccessMessage(null),
                                  3000
                                );
                              }
                            }
                          } catch (err) {
                            setError(
                              `Error al seleccionar carpeta: ${getErrorMessage(err)}`
                            );
                          }
                        }}
                      >
                        📁 Seleccionar
                      </button>
                    </div>
                  </div>
                </div>

                {/* Backup automático habilitado */}
                <div class="space-y-4 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <div class="flex items-center justify-between">
                    <div>
                      <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                        Backup Automático
                      </h3>
                      <p class="text-sm text-gray-600 dark:text-gray-400">
                        Crea un backup al abrir la app si pasó el intervalo
                        desde el último.
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
                            persistBackupConfig({
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
                        onChange={(e) => {
                          const cfg = config();
                          if (cfg) {
                            // Guarda de NaN + clamp a [1, 30]; persiste al confirmar.
                            const n = parseInt(e.currentTarget.value, 10);
                            const days = Number.isNaN(n)
                              ? 7
                              : Math.min(30, Math.max(1, n));
                            persistBackupConfig({
                              ...cfg,
                              backup: {
                                ...cfg.backup,
                                auto_backup_interval: days,
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
                            // Guarda de NaN: input vacío → NaN rompería la
                            // deserialización a u32 en el backend y fallaría el guardado.
                            const n = parseInt(e.currentTarget.value, 10);
                            setConfig({
                              ...cfg,
                              backup: {
                                ...cfg.backup,
                                retention_days: Number.isNaN(n) ? 30 : n,
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
                        const mode = e.currentTarget.value as ThemeMode;
                        // El tema se aplica y persiste EN EL ACTO vía el contexto:
                        // antes solo se guardaba en la config y no lo leía nadie,
                        // así que elegir "Oscuro" acá no cambiaba nada en pantalla.
                        setThemeMode(mode);
                        const cfg = config();
                        if (cfg) {
                          setConfig({ ...cfg, ui: { ...cfg.ui, theme: mode } });
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
                        Pedir confirmación al eliminar proyectos (las
                        eliminaciones definitivas siempre piden confirmación)
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

            {/* Tab: Atajos de Teclado */}
            <Show when={activeTab() === 'shortcuts'}>
              <div class="space-y-6">
                <p class="mb-4 text-gray-600 dark:text-gray-400">
                  Configura los atajos de teclado para acciones rápidas.
                </p>

                {/* Habilitar/Deshabilitar shortcuts */}
                <div class="flex items-center justify-between rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <div>
                    <p class="font-medium text-gray-900 dark:text-white">
                      Habilitar Atajos de Teclado
                    </p>
                    <p class="text-sm text-gray-600 dark:text-gray-400">
                      Activar shortcuts globales en toda la aplicación
                    </p>
                  </div>
                  <label class="relative inline-flex cursor-pointer items-center">
                    <input
                      type="checkbox"
                      class="peer sr-only"
                      checked={config()?.shortcuts.enabled ?? true}
                      onChange={(e) => {
                        const cfg = config();
                        if (cfg) {
                          setConfig({
                            ...cfg,
                            shortcuts: {
                              ...cfg.shortcuts,
                              enabled: e.currentTarget.checked,
                            },
                          });
                        }
                      }}
                    />
                    <div class="peer h-6 w-11 rounded-full bg-gray-200 after:absolute after:start-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:border-gray-600 dark:bg-gray-700 dark:peer-focus:ring-blue-800 rtl:peer-checked:after:-translate-x-full" />
                  </label>
                </div>

                {/* Lista de atajos configurables */}
                <div class="space-y-4">
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                    Atajos Disponibles
                  </h3>

                  {/* Un solo bloque parametrizado: los seis atajos rendereaban
                      markup IDÉNTICO y solo cambiaban clave, textos y tecla por
                      defecto. Ver SHORTCUT_METADATA en types/config.ts. */}
                  <For each={Object.entries(SHORTCUT_METADATA)}>
                    {([action, meta]) => (
                      <div class="rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800">
                        <div class="mb-2 flex items-center justify-between">
                          <div>
                            <p class="font-medium text-gray-900 dark:text-white">
                              {meta.title}
                            </p>
                            <p class="text-sm text-gray-600 dark:text-gray-400">
                              {meta.description}
                            </p>
                          </div>
                          <label class="relative inline-flex cursor-pointer items-center">
                            <input
                              type="checkbox"
                              class="peer sr-only"
                              checked={
                                config()?.shortcuts.shortcuts[action]?.enabled ??
                                true
                              }
                              onChange={(e) => {
                                const cfg = config();
                                if (cfg) {
                                  setConfig({
                                    ...cfg,
                                    shortcuts: {
                                      ...cfg.shortcuts,
                                      shortcuts: {
                                        ...cfg.shortcuts.shortcuts,
                                        [action]: {
                                          ...cfg.shortcuts.shortcuts[action],
                                          enabled: e.currentTarget.checked,
                                        },
                                      },
                                    },
                                  });
                                }
                              }}
                            />
                            <div class="peer h-6 w-11 rounded-full bg-gray-200 after:absolute after:start-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-green-600 peer-checked:after:translate-x-full peer-checked:after:border-white peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-green-300 dark:border-gray-600 dark:bg-gray-700 dark:peer-focus:ring-green-800 rtl:peer-checked:after:-translate-x-full" />
                          </label>
                        </div>
                        <div class="flex items-center gap-2">
                          <kbd class="rounded border border-gray-300 bg-gray-100 px-2 py-1 font-mono text-xs text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white">
                            {config()?.shortcuts.shortcuts[action]?.key ||
                              meta.defaultKey}
                          </kbd>
                        </div>
                      </div>
                    )}
                  </For>
                </div>

                <div class="rounded-lg bg-blue-50 p-4 dark:bg-blue-900/20">
                  <p class="text-sm text-blue-800 dark:text-blue-200">
                    💡 <strong>Tip:</strong> Los atajos se activan cuando la
                    aplicación está en foco. Usa{' '}
                    <kbd class="rounded bg-blue-100 px-1 font-mono text-xs dark:bg-blue-800">
                      Ctrl
                    </kbd>{' '}
                    (Linux/Windows) o{' '}
                    <kbd class="rounded bg-blue-100 px-1 font-mono text-xs dark:bg-blue-800">
                      Cmd
                    </kbd>{' '}
                    (macOS) según tu sistema operativo.
                  </p>
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
              onClick={() => props.onClose()}
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
