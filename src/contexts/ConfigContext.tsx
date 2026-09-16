import {
  createContext,
  createSignal,
  onMount,
  useContext,
  type ParentComponent,
} from 'solid-js';
import type { AppConfig, UiConfig } from '../types/config';
import { getConfig, updateConfig } from '../services/api';
import { getErrorMessage } from '../utils/errors';

/**
 * Fuente ÚNICA de verdad de la configuración de la app en el frontend.
 *
 * Antes cada consumidor llamaba a `getConfig()` por su cuenta (o directamente
 * no la leía: `ui.theme` y `ui.confirm_delete` se escribían en Settings y no
 * los leía NADIE). Centralizarla acá es lo que permite que el tema y las
 * confirmaciones de borrado respondan de verdad a lo que el usuario eligió.
 */
export interface ConfigContextValue {
  /** Config viva. `null` mientras todavía no resolvió `get_config`. */
  config: () => AppConfig | null;
  /** `true` una vez que el primer `get_config` terminó (con éxito o error). */
  isLoaded: () => boolean;
  error: () => string | null;
  reload: () => Promise<void>;
  /** Persiste un parche sobre `ui` y actualiza la config viva. */
  patchUi: (patch: Partial<UiConfig>) => Promise<void>;
}

const ConfigContext = createContext<ConfigContextValue>();

export const ConfigProvider: ParentComponent = (props) => {
  const [config, setConfig] = createSignal<AppConfig | null>(null);
  const [isLoaded, setIsLoaded] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const reload = async () => {
    try {
      setConfig(await getConfig());
      setError(null);
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setIsLoaded(true);
    }
  };

  onMount(reload);

  const patchUi = async (patch: Partial<UiConfig>) => {
    // Releemos del disco antes de escribir, igual que hace Settings con `backup`:
    // así un parche de `ui` no pisa cambios que otra parte haya persistido mientras
    // esta config vivía en memoria.
    const persisted = await getConfig();
    const next: AppConfig = {
      ...persisted,
      ui: { ...persisted.ui, ...patch },
    };
    await updateConfig(next);
    setConfig(next);
  };

  const value: ConfigContextValue = {
    config,
    isLoaded,
    error,
    reload,
    patchUi,
  };

  return (
    <ConfigContext.Provider value={value}>
      {props.children}
    </ConfigContext.Provider>
  );
};

/**
 * Fallback para cuando NO hay provider (típicamente un test que monta un
 * componente suelto). A propósito NO tira una excepción: los consumidores de
 * esta config gobiernan caminos DESTRUCTIVOS, y sin config la decisión segura
 * es "comportate como el default" (que es confirmar siempre), no explotar ni
 * saltear la confirmación.
 */
const FALLBACK: ConfigContextValue = {
  config: () => null,
  isLoaded: () => false,
  error: () => null,
  reload: async () => {},
  patchUi: async () => {},
};

export const useConfig = (): ConfigContextValue =>
  useContext(ConfigContext) ?? FALLBACK;
