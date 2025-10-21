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
  const [registeredShortcuts, setRegisteredShortcuts] = createSignal<string[]>(
    []
  );

  // Cargar configuración de shortcuts al montar
  onMount(async () => {
    try {
      const shortcutsConfig = await getShortcutsConfig();
      setConfig(shortcutsConfig);
      console.log('⚙️ [SHORTCUTS] Configuración cargada');
    } catch (error) {
      console.error('Error cargando configuración de shortcuts:', error);
    }
  });

  // Limpiar shortcuts al desmontar
  onCleanup(async () => {
    try {
      await unregisterAll();
      console.log('✅ [SHORTCUTS] Todos los atajos limpiados');
    } catch (error) {
      console.error('Error limpiando shortcuts:', error);
    }
  });

  // Registrar todos los shortcuts desde la configuración
  const registerAllShortcuts = async (shortcutsConfig: ShortcutsConfig) => {
    const registered: string[] = [];

    console.log('🔍 [SHORTCUTS] registerAllShortcuts iniciado');
    console.log(
      '🔍 [SHORTCUTS] Shortcuts en config:',
      Object.keys(shortcutsConfig.shortcuts)
    );

    for (const [action, binding] of Object.entries(shortcutsConfig.shortcuts)) {
      console.log(
        `🔍 [SHORTCUTS] Procesando ${action}: enabled=${binding.enabled}, key=${binding.key}`
      );

      if (binding.enabled) {
        try {
          console.log(`📝 [SHORTCUTS] Intentando registrar ${binding.key}...`);
          await register(binding.key, async () => {
            console.log(`⌨️ [SHORTCUT] Ejecutando acción: ${action}`);
            const handler = handlers().get(action as ShortcutAction);
            if (handler) {
              await handler();
            } else {
              console.warn(
                `⚠️ [SHORTCUT] No hay handler registrado para: ${action}`
              );
            }
          });
          registered.push(binding.key);
          console.log(`✅ [SHORTCUT] Registrado: ${binding.key} → ${action}`);
        } catch (error) {
          console.error(
            `❌ [SHORTCUT] Error registrando ${binding.key}:`,
            error
          );
        }
      } else {
        console.log(`⏭️ [SHORTCUTS] Saltando ${action} (deshabilitado)`);
      }
    }

    console.log(`✅ [SHORTCUTS] Total registrados: ${registered.length}`);
    setRegisteredShortcuts(registered);
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
    console.log(`📝 [SHORTCUT] Handler registrado para: ${action}`);
  };

  // Desregistrar un handler
  const unregisterHandler = (action: ShortcutAction) => {
    setHandlers((prev) => {
      const newHandlers = new Map(prev);
      newHandlers.delete(action);
      return newHandlers;
    });
    console.log(`🗑️ [SHORTCUT] Handler eliminado para: ${action}`);
  };

  const isEnabled = () => config()?.enabled ?? false;

  // Re-registrar shortcuts (llamar cuando todos los handlers estén listos)
  const reregisterShortcuts = async () => {
    const cfg = config();
    const handlersMap = handlers();

    console.log(
      `🔄 [SHORTCUTS] Re-registrando con ${handlersMap.size} handlers disponibles`
    );
    console.log(`🔄 [SHORTCUTS] Config:`, cfg);
    console.log(`🔄 [SHORTCUTS] Enabled:`, cfg?.enabled);

    if (!cfg) {
      console.warn('⚠️ [SHORTCUTS] No hay configuración disponible');
      return;
    }

    if (!cfg.enabled) {
      console.warn('⚠️ [SHORTCUTS] Shortcuts deshabilitados en configuración');
      return;
    }

    if (handlersMap.size === 0) {
      console.warn('⚠️ [SHORTCUTS] No hay handlers registrados');
      return;
    }

    // Limpiar shortcuts existentes primero
    try {
      console.log('🧹 [SHORTCUTS] Limpiando shortcuts existentes...');
      await unregisterAll();
      console.log('✅ [SHORTCUTS] Shortcuts limpiados');
    } catch (error) {
      console.error(
        '❌ [SHORTCUTS] Error limpiando shortcuts anteriores:',
        error
      );
    }

    // Registrar nuevamente
    console.log('📝 [SHORTCUTS] Iniciando registro de shortcuts...');
    await registerAllShortcuts(cfg);
    console.log('✅ [SHORTCUTS] Registro completado');
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
