import {
  createContext,
  useContext,
  ParentComponent,
  onMount,
  onCleanup,
  createSignal,
} from 'solid-js';
import { register, unregisterAll } from '@tauri-apps/plugin-global-shortcut';
import type { ShortcutsConfig, ShortcutAction } from '../types/config';
import { getShortcutsConfig } from '../services/api';
import { logger } from '../utils/logger';

// Tipo para los handlers de shortcuts
type ShortcutHandler = () => void | Promise<void>;

interface ShortcutsContextValue {
  config: () => ShortcutsConfig | null;
  registerHandler: (action: ShortcutAction, handler: ShortcutHandler) => void;
  unregisterHandler: (action: ShortcutAction) => void;
  isEnabled: () => boolean;
  reregisterShortcuts: () => Promise<void>;
}

const ShortcutsContext = createContext<ShortcutsContextValue>();

export const ShortcutsProvider: ParentComponent = (props) => {
  const [config, setConfig] = createSignal<ShortcutsConfig | null>(null);
  const [handlers, setHandlers] = createSignal<
    Map<ShortcutAction, ShortcutHandler>
  >(new Map());

  // Cargar configuración de shortcuts al montar
  onMount(async () => {
    try {
      const shortcutsConfig = await getShortcutsConfig();
      setConfig(shortcutsConfig);
      logger.debug('⚙️ [SHORTCUTS] Configuración cargada');
    } catch (error) {
      logger.error('Error cargando configuración de shortcuts:', error);
    }
  });

  // Limpiar shortcuts al desmontar
  onCleanup(async () => {
    try {
      await unregisterAll();
      logger.debug('✅ [SHORTCUTS] Todos los atajos limpiados');
    } catch (error) {
      logger.error('Error limpiando shortcuts:', error);
    }
  });

  // Registrar todos los shortcuts desde la configuración
  const registerAllShortcuts = async (shortcutsConfig: ShortcutsConfig) => {
    const registered: string[] = [];

    logger.debug('🔍 [SHORTCUTS] registerAllShortcuts iniciado');
    logger.debug(
      '🔍 [SHORTCUTS] Shortcuts en config:',
      Object.keys(shortcutsConfig.shortcuts)
    );

    for (const [action, binding] of Object.entries(shortcutsConfig.shortcuts)) {
      logger.debug(
        `🔍 [SHORTCUTS] Procesando ${action}: enabled=${binding.enabled}, key=${binding.key}`
      );

      if (binding.enabled) {
        try {
          logger.debug(`📝 [SHORTCUTS] Intentando registrar ${binding.key}...`);
          await register(binding.key, async () => {
            logger.debug(`⌨️ [SHORTCUT] Ejecutando acción: ${action}`);
            const handler = handlers().get(action as ShortcutAction);
            if (handler) {
              await handler();
            } else {
              logger.warn(
                `⚠️ [SHORTCUT] No hay handler registrado para: ${action}`
              );
            }
          });
          registered.push(binding.key);
          logger.debug(`✅ [SHORTCUT] Registrado: ${binding.key} → ${action}`);
        } catch (error) {
          logger.error(
            `❌ [SHORTCUT] Error registrando ${binding.key}:`,
            error
          );
        }
      } else {
        logger.debug(`⏭️ [SHORTCUTS] Saltando ${action} (deshabilitado)`);
      }
    }

    logger.debug(`✅ [SHORTCUTS] Total registrados: ${registered.length}`);
  };

  // Registrar un handler para una acción
  const registerHandler = (
    action: ShortcutAction,
    handler: ShortcutHandler
  ) => {
    setHandlers((prev) => {
      const newHandlers = new Map(prev);
      newHandlers.set(action, handler);
      return newHandlers;
    });
    logger.debug(`📝 [SHORTCUT] Handler registrado para: ${action}`);
  };

  // Desregistrar un handler
  const unregisterHandler = (action: ShortcutAction) => {
    setHandlers((prev) => {
      const newHandlers = new Map(prev);
      newHandlers.delete(action);
      return newHandlers;
    });
    logger.debug(`🗑️ [SHORTCUT] Handler eliminado para: ${action}`);
  };

  const isEnabled = () => config()?.enabled ?? false;

  // Re-registrar shortcuts (llamar cuando todos los handlers estén listos)
  const reregisterShortcuts = async () => {
    const cfg = config();
    const handlersMap = handlers();

    logger.debug(
      `🔄 [SHORTCUTS] Re-registrando con ${handlersMap.size} handlers disponibles`
    );
    logger.debug(`🔄 [SHORTCUTS] Config:`, cfg);
    logger.debug(`🔄 [SHORTCUTS] Enabled:`, cfg?.enabled);

    if (!cfg) {
      logger.warn('⚠️ [SHORTCUTS] No hay configuración disponible');
      return;
    }

    if (!cfg.enabled) {
      logger.warn('⚠️ [SHORTCUTS] Shortcuts deshabilitados en configuración');
      return;
    }

    if (handlersMap.size === 0) {
      logger.warn('⚠️ [SHORTCUTS] No hay handlers registrados');
      return;
    }

    // Limpiar shortcuts existentes primero
    try {
      logger.debug('🧹 [SHORTCUTS] Limpiando shortcuts existentes...');
      await unregisterAll();
      logger.debug('✅ [SHORTCUTS] Shortcuts limpiados');
    } catch (error) {
      logger.error(
        '❌ [SHORTCUTS] Error limpiando shortcuts anteriores:',
        error
      );
    }

    // Registrar nuevamente
    logger.debug('📝 [SHORTCUTS] Iniciando registro de shortcuts...');
    await registerAllShortcuts(cfg);
    logger.debug('✅ [SHORTCUTS] Registro completado');
  };

  const contextValue: ShortcutsContextValue = {
    config,
    registerHandler,
    unregisterHandler,
    isEnabled,
    reregisterShortcuts,
  };

  return (
    <ShortcutsContext.Provider value={contextValue}>
      {props.children}
    </ShortcutsContext.Provider>
  );
};

// Hook para usar el contexto de shortcuts
export const useShortcuts = () => {
  const context = useContext(ShortcutsContext);
  if (!context) {
    throw new Error('useShortcuts debe usarse dentro de ShortcutsProvider');
  }
  return context;
};
