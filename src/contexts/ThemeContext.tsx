import {
  createContext,
  createEffect,
  createSignal,
  onCleanup,
  useContext,
  type ParentComponent,
} from 'solid-js';
import type { ThemeMode } from '../types/config';
import { useConfig } from './ConfigContext';
import { getErrorMessage } from '../utils/errors';
import { logger } from '../utils/logger';

// El tipo vive en types/config.ts porque es el MISMO que viaja en la config de
// Rust (`ui.theme`). Antes acá había un `Theme = 'light' | 'dark'` paralelo que
// no admitía 'auto', así que el select de Settings ofrecía un modo que el
// contexto no sabía representar.
export type { ThemeMode };

/** Lo que realmente se aplica al DOM. 'auto' se resuelve a uno de estos dos. */
export type ResolvedTheme = 'light' | 'dark';

const STORAGE_KEY = 'theme';
const DARK_QUERY = '(prefers-color-scheme: dark)';

interface ThemeContextValue {
  /** Preferencia elegida por el usuario: 'light' | 'dark' | 'auto'. */
  themeMode: () => ThemeMode;
  /** Tema efectivo aplicado al DOM: 'light' | 'dark'. */
  resolvedTheme: () => ResolvedTheme;
  setThemeMode: (mode: ThemeMode) => Promise<void>;
}

const ThemeContext = createContext<ThemeContextValue>();

const isThemeMode = (value: unknown): value is ThemeMode =>
  value === 'light' || value === 'dark' || value === 'auto';

/**
 * Lee el caché de arranque. OJO con el rol de localStorage acá: NO es la fuente
 * de verdad (esa es la config de Rust), es solo un caché para pintar el tema
 * correcto en el primer frame. `localStorage` es sincrónico y `invoke()` no, así
 * que sin este caché la app arrancaría siempre en claro y pegaría un flash al
 * resolver `get_config`.
 */
const readCachedMode = (): ThemeMode => {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (isThemeMode(saved)) return saved;
  } catch {
    // localStorage puede no estar disponible; seguimos con el default.
  }
  // 'auto' es el mismo default que usa el backend (schema.rs) y el select de Settings.
  return 'auto';
};

const systemPrefersDark = (): boolean => {
  if (typeof window === 'undefined' || !window.matchMedia) return false;
  return window.matchMedia(DARK_QUERY).matches;
};

export const ThemeProvider: ParentComponent = (props) => {
  const configCtx = useConfig();

  const [themeMode, setThemeModeSignal] =
    createSignal<ThemeMode>(readCachedMode());
  const [prefersDark, setPrefersDark] = createSignal(systemPrefersDark());

  const resolvedTheme = (): ResolvedTheme => {
    const mode = themeMode();
    if (mode === 'auto') return prefersDark() ? 'dark' : 'light';
    return mode;
  };

  // Listener VIVO del sistema: en modo 'auto' el tema tiene que seguir cambiando
  // si el usuario cambia el esquema del SO con la app abierta. Una lectura única
  // al montar dejaría el tema congelado.
  if (typeof window !== 'undefined' && window.matchMedia) {
    const mql = window.matchMedia(DARK_QUERY);
    // Tipado estructural a propósito: `MediaQueryListEvent` no está en la
    // allowlist de globals de ESLint y lo único que necesitamos es `matches`.
    const onChange = (event: { matches: boolean }) =>
      setPrefersDark(event.matches);
    mql.addEventListener('change', onChange);
    onCleanup(() => mql.removeEventListener('change', onChange));
  }

  // Aplicar al DOM y refrescar el caché de arranque.
  createEffect(() => {
    document.documentElement.classList.toggle('dark', resolvedTheme() === 'dark');
    try {
      localStorage.setItem(STORAGE_KEY, themeMode());
    } catch {
      // Sin localStorage perdemos solo el caché anti-flash, no la persistencia real.
    }
  });

  // Reconciliación: cuando llega la config de Rust (la fuente de verdad), manda ella.
  createEffect(() => {
    const cfg = configCtx.config();
    if (cfg) setThemeModeSignal(cfg.ui.theme);
  });

  const setThemeMode = async (mode: ThemeMode) => {
    // Optimista: la UI responde en el acto y después persistimos.
    setThemeModeSignal(mode);
    try {
      await configCtx.patchUi({ theme: mode });
    } catch (err) {
      // No revertimos: el tema ya aplicado es lo que el usuario pidió. Lo que se
      // pierde es la persistencia, y el próximo cambio exitoso la recupera.
      logger.error('No se pudo persistir el tema:', getErrorMessage(err));
    }
  };

  const value: ThemeContextValue = {
    themeMode,
    resolvedTheme,
    setThemeMode,
  };

  return (
    <ThemeContext.Provider value={value}>
      {props.children}
    </ThemeContext.Provider>
  );
};

export const useTheme = () => {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
};
