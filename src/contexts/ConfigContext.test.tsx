import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import type { JSX } from 'solid-js';
import { ConfigProvider, useConfig } from './ConfigContext';
import type { AppConfig } from '../types/config';

vi.mock('../services/api', () => ({
  getConfig: vi.fn(),
  updateConfig: vi.fn(),
}));

import { getConfig, updateConfig } from '../services/api';

const baseConfig = (): AppConfig =>
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
    ui: { theme: 'auto', language: 'es', confirm_delete: true, show_welcome: true },
    advanced: {
      log_level: 'info',
      enable_analytics: true,
      enable_auto_update: true,
    },
    shortcuts: { enabled: true, shortcuts: {} },
  }) as AppConfig;

function Probe(): JSX.Element {
  const { config, isLoaded, error, patchUi } = useConfig();
  return (
    <div>
      <span data-testid="loaded">{String(isLoaded())}</span>
      <span data-testid="confirm-delete">
        {String(config()?.ui.confirm_delete ?? 'sin-config')}
      </span>
      <span data-testid="error">{error() ?? ''}</span>
      <button onClick={() => patchUi({ confirm_delete: false })}>apagar</button>
    </div>
  );
}

describe('ConfigContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getConfig).mockResolvedValue(baseConfig());
    vi.mocked(updateConfig).mockResolvedValue(undefined);
  });

  it('carga la config al montar', async () => {
    render(() => (
      <ConfigProvider>
        <Probe />
      </ConfigProvider>
    ));

    await waitFor(() =>
      expect(screen.getByTestId('loaded').textContent).toBe('true')
    );
    expect(screen.getByTestId('confirm-delete').textContent).toBe('true');
    expect(getConfig).toHaveBeenCalledTimes(1);
  });

  it('patchUi persiste solo el parche de ui y conserva el resto', async () => {
    render(() => (
      <ConfigProvider>
        <Probe />
      </ConfigProvider>
    ));

    await waitFor(() =>
      expect(screen.getByTestId('loaded').textContent).toBe('true')
    );

    screen.getByRole('button', { name: 'apagar' }).click();

    await waitFor(() => expect(updateConfig).toHaveBeenCalledTimes(1));
    const persisted = vi.mocked(updateConfig).mock.calls[0][0];
    expect(persisted.ui.confirm_delete).toBe(false);
    // El parche NO puede pisar el resto de ui ni las otras secciones.
    expect(persisted.ui.theme).toBe('auto');
    expect(persisted.ui.language).toBe('es');
    expect(persisted.backup.retention_days).toBe(30);

    await waitFor(() =>
      expect(screen.getByTestId('confirm-delete').textContent).toBe('false')
    );
  });

  it('expone el error y marca isLoaded si get_config falla', async () => {
    vi.mocked(getConfig).mockRejectedValue('No se pudo leer la configuración.');

    render(() => (
      <ConfigProvider>
        <Probe />
      </ConfigProvider>
    ));

    await waitFor(() =>
      expect(screen.getByTestId('loaded').textContent).toBe('true')
    );
    expect(screen.getByTestId('error').textContent).toBe(
      'No se pudo leer la configuración.'
    );
    expect(screen.getByTestId('confirm-delete').textContent).toBe('sin-config');
  });

  it('sin provider devuelve el fallback en vez de explotar', async () => {
    // Un componente suelto (típico en tests) no debe romperse: devuelve config
    // null, y los caminos destructivos caen en el default de "confirmar siempre".
    render(() => <Probe />);

    expect(screen.getByTestId('confirm-delete').textContent).toBe('sin-config');
    expect(screen.getByTestId('loaded').textContent).toBe('false');
    expect(getConfig).not.toHaveBeenCalled();
  });
});
