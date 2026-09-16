import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import type { JSX } from 'solid-js';
import { ThemeProvider, useTheme } from './ThemeContext';
import { ConfigProvider } from './ConfigContext';
import { setSystemPrefersDark } from '../test-setup';
import type { AppConfig, ThemeMode } from '../types/config';

vi.mock('../services/api', () => ({
  getConfig: vi.fn(),
  updateConfig: vi.fn(),
}));

import { getConfig, updateConfig } from '../services/api';

const baseConfig = (theme: ThemeMode): AppConfig =>
  ({
    version: '0.6.0',
    platform: {
      os_override: 'auto',
      terminal: { mode: 'auto', custom_args: [] },
      browser: { mode: 'auto', custom_args: [] },
      file_manager: { mode: 'auto', custom_args: [] },
      text_editor: { mode: 'auto', custom_args: [] },
      environment: {},
    },
    backup: {
      auto_backup_enabled: false,
      auto_backup_interval: 24,
      cleanup_old_backups: false,
      retention_days: 30,
    },
    ui: { theme, language: 'es', confirm_delete: true, show_welcome: false },
    advanced: {
      log_level: 'info',
      enable_analytics: true,
      enable_auto_update: true,
    },
    shortcuts: { enabled: true, shortcuts: {} },
  }) as AppConfig;

// Sonda: expone el contexto en el DOM para poder afirmar sobre él.
function Probe(): JSX.Element {
  const { themeMode, resolvedTheme, setThemeMode } = useTheme();
  return (
    <div>
      <span data-testid="mode">{themeMode()}</span>
      <span data-testid="resolved">{resolvedTheme()}</span>
      <button onClick={() => setThemeMode('dark')}>a oscuro</button>
    </div>
  );
}

const renderWithProviders = () =>
  render(() => (
    <ConfigProvider>
      <ThemeProvider>
        <Probe />
      </ThemeProvider>
    </ConfigProvider>
  ));

const hasDarkClass = () => document.documentElement.classList.contains('dark');

describe('ThemeContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(updateConfig).mockResolvedValue(undefined);
  });

  it('aplica la clase dark cuando la preferencia persistida es dark', async () => {
    vi.mocked(getConfig).mockResolvedValue(baseConfig('dark'));

    renderWithProviders();

    await waitFor(() => expect(screen.getByTestId('mode').textContent).toBe('dark'));
    expect(screen.getByTestId('resolved').textContent).toBe('dark');
    expect(hasDarkClass()).toBe(true);
  });

  it('NO aplica la clase dark cuando la preferencia persistida es light', async () => {
    // Sistema en oscuro a propósito: una preferencia explícita 'light' tiene que
    // ganarle al sistema, no dejarse arrastrar.
    setSystemPrefersDark(true);
    vi.mocked(getConfig).mockResolvedValue(baseConfig('light'));

    renderWithProviders();

    await waitFor(() =>
      expect(screen.getByTestId('mode').textContent).toBe('light')
    );
    expect(screen.getByTestId('resolved').textContent).toBe('light');
    expect(hasDarkClass()).toBe(false);
  });

  it('en modo auto sigue al sistema y REACCIONA a un cambio en vivo', async () => {
    setSystemPrefersDark(true);
    vi.mocked(getConfig).mockResolvedValue(baseConfig('auto'));

    renderWithProviders();

    await waitFor(() =>
      expect(screen.getByTestId('mode').textContent).toBe('auto')
    );
    expect(screen.getByTestId('resolved').textContent).toBe('dark');
    expect(hasDarkClass()).toBe(true);

    // El sistema pasa a claro con la app abierta: una lectura única dejaría el
    // tema congelado en oscuro. El listener vivo tiene que corregirlo.
    setSystemPrefersDark(false);

    await waitFor(() => expect(hasDarkClass()).toBe(false));
    expect(screen.getByTestId('resolved').textContent).toBe('light');
    // La PREFERENCIA sigue siendo 'auto': cambió el sistema, no lo que eligió el usuario.
    expect(screen.getByTestId('mode').textContent).toBe('auto');
  });

  it('persiste el cambio de tema en la config con el ui.theme correcto', async () => {
    vi.mocked(getConfig).mockResolvedValue(baseConfig('light'));

    renderWithProviders();

    await waitFor(() =>
      expect(screen.getByTestId('mode').textContent).toBe('light')
    );

    screen.getByRole('button', { name: 'a oscuro' }).click();

    await waitFor(() => expect(updateConfig).toHaveBeenCalledTimes(1));
    const persisted = vi.mocked(updateConfig).mock.calls[0][0];
    expect(persisted.ui.theme).toBe('dark');
    // El resto de la config no se toca al parchear solo el tema.
    expect(persisted.ui.confirm_delete).toBe(true);
    expect(persisted.version).toBe('0.6.0');

    await waitFor(() => expect(hasDarkClass()).toBe(true));
  });

  it('useTheme explota si no hay ThemeProvider', () => {
    expect(() => render(() => <Probe />)).toThrow(/ThemeProvider/);
  });
});
