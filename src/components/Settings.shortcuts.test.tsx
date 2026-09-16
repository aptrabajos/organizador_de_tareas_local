import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import userEvent from '@testing-library/user-event';
import Settings from './Settings';
import { ConfigProvider } from '../contexts/ConfigContext';
import { ThemeProvider } from '../contexts/ThemeContext';
import type { AppConfig } from '../types/config';

/**
 * Red del refactor de la pestaña "Atajos".
 *
 * Los seis bloques copiados (~290 líneas) se colapsaron en un `<For>` sobre
 * `SHORTCUT_METADATA`. `Settings.tsx` no tenía NINGÚN test, así que ese refactor
 * salió sin red: esto la pone. Lo que se fija es el contrato visible de la
 * pestaña — los seis atajos, con su título, su descripción y su tecla por
 * defecto, EN ORDEN. Si la plantilla pierde un texto o cambia el orden, falla.
 */

vi.mock('../services/api', async () => {
  const actual =
    await vi.importActual<typeof import('../services/api')>('../services/api');
  return {
    ...actual,
    getConfig: vi.fn(),
    updateConfig: vi.fn(),
    detectPrograms: vi.fn(),
    listBackups: vi.fn(),
  };
});

import {
  getConfig,
  updateConfig,
  detectPrograms,
  listBackups,
} from '../services/api';

// Config sin la sección `shortcuts.shortcuts` poblada: así los seis <kbd> caen
// en la tecla por defecto, que es justo lo que antes estaba hardcodeado en el JSX.
const emptyShortcutsConfig = (): AppConfig =>
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

// Copiado LITERAL de los seis bloques previos al refactor, en su orden original.
const EXPECTED = [
  ['Nuevo Proyecto', 'Abrir formulario de nuevo proyecto', 'Ctrl+N'],
  ['Buscar Proyectos', 'Focus en barra de búsqueda', 'Ctrl+F'],
  ['Abrir Configuración', 'Abrir este panel de configuración', 'Ctrl+Comma'],
  ['Acerca de', 'Ver información de la aplicación', 'Ctrl+Shift+A'],
  ['Recargar Proyectos', 'Actualizar lista de proyectos', 'Ctrl+R'],
  ['Cerrar Modal', 'Cerrar cualquier modal activo', 'Escape'],
];

const openShortcutsTab = async () => {
  const user = userEvent.setup();
  render(() => (
    <ConfigProvider>
      <ThemeProvider>
        <Settings onClose={() => {}} />
      </ThemeProvider>
    </ConfigProvider>
  ));

  await waitFor(() => expect(getConfig).toHaveBeenCalled());
  await user.click(screen.getByRole('button', { name: /atajos/i }));
  await screen.findByText('Atajos Disponibles');
  return user;
};

describe('Settings — pestaña de atajos', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getConfig).mockResolvedValue(emptyShortcutsConfig());
    vi.mocked(updateConfig).mockResolvedValue(undefined);
    vi.mocked(detectPrograms).mockResolvedValue({
      terminals: [],
      browsers: [],
      file_managers: [],
      text_editors: [],
    });
    vi.mocked(listBackups).mockResolvedValue([]);
  });

  it('renderiza los seis atajos con título, descripción y tecla por defecto', async () => {
    await openShortcutsTab();

    for (const [title, description, key] of EXPECTED) {
      expect(screen.getByText(title), `falta el título "${title}"`).toBeTruthy();
      expect(
        screen.getByText(description),
        `falta la descripción "${description}"`
      ).toBeTruthy();
      expect(screen.getByText(key), `falta la tecla "${key}"`).toBeTruthy();
    }
  });

  it('respeta el orden original de los atajos', async () => {
    await openShortcutsTab();

    const titles = EXPECTED.map(([title]) => title);
    const positions = titles.map((title) =>
      // compareDocumentPosition da el orden real en el DOM, no el de la query.
      Array.from(document.querySelectorAll('p')).findIndex(
        (p) => p.textContent === title
      )
    );

    expect(positions.every((pos) => pos >= 0)).toBe(true);
    expect([...positions]).toEqual([...positions].sort((a, b) => a - b));
  });

  it('usa la tecla configurada en vez de la de por defecto cuando existe', async () => {
    const cfg = emptyShortcutsConfig();
    cfg.shortcuts.shortcuts = {
      new_project: { key: 'Ctrl+Alt+N', enabled: true },
    };
    vi.mocked(getConfig).mockResolvedValue(cfg);

    await openShortcutsTab();

    expect(screen.getByText('Ctrl+Alt+N')).toBeTruthy();
    expect(screen.queryByText('Ctrl+N')).toBeFalsy();
    // Los otros cinco siguen mostrando su default.
    expect(screen.getByText('Ctrl+F')).toBeTruthy();
  });

  it('el toggle de un atajo no pisa la configuración de los otros', async () => {
    const user = await openShortcutsTab();

    const toggles = screen
      .getAllByRole('checkbox')
      .filter((el) => (el as HTMLInputElement).checked);
    expect(toggles.length).toBeGreaterThanOrEqual(6);

    // Apagamos el primer atajo de la lista (new_project).
    const container = screen.getByText('Nuevo Proyecto').closest('div.rounded-lg');
    const checkbox = container!.querySelector(
      'input[type="checkbox"]'
    ) as HTMLInputElement;
    await user.click(checkbox);

    await waitFor(() => expect(checkbox.checked).toBe(false));
    // El resto sigue encendido: el spread del handler no puede aplanar los demás.
    const stillOn = screen
      .getAllByRole('checkbox')
      .filter((el) => (el as HTMLInputElement).checked);
    expect(stillOn.length).toBeGreaterThanOrEqual(5);
  });
});
