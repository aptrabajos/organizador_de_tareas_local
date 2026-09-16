import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import userEvent from '@testing-library/user-event';
import TrashModal from './TrashModal';
import { ConfigProvider } from '../contexts/ConfigContext';
import { setUiConfigOverrides } from '../test-setup';
import type { ProjectStore } from '../stores/projectStore';
import type { TrashItem } from '../types/project';

/**
 * Guardia del camino IRREVERSIBLE.
 *
 * Purgar y vaciar la papelera confirman SIEMPRE, sin consultar `ui.confirm_delete`.
 * Hoy TrashModal ni siquiera lee la config, así que estos tests parecen triviales
 * — y esa es justamente la razón de que existan: lo que protegen es que nadie
 * "unifique el criterio" más adelante y gatee estas dos operaciones con el flag.
 * Apagar una preferencia de comodidad no puede habilitar la pérdida definitiva
 * de datos con un click. Ver src/utils/confirm.ts.
 */

// OJO: el mock global de test-setup para plugin-dialog expone `open` y `save`
// pero NO `confirm`, que es justo lo que usa TrashModal. Lo agregamos acá.
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(() => Promise.resolve(null)),
  save: vi.fn(() => Promise.resolve(null)),
  confirm: vi.fn(),
}));

vi.mock('../services/api', async () => {
  const actual =
    await vi.importActual<typeof import('../services/api')>('../services/api');
  return { ...actual, listTrash: vi.fn() };
});

import { confirm } from '@tauri-apps/plugin-dialog';
import { listTrash } from '../services/api';

const trashItems: TrashItem[] = [
  {
    id: 7,
    name: 'Proyecto borrado',
    deleted_at: '2025-01-03T00:00:00Z',
    subproject_count: 0,
  },
];

const purgeProject = vi.fn();
const emptyTrash = vi.fn();

// Store mínimo: TrashModal solo usa estas tres operaciones.
const fakeStore = () =>
  ({
    purgeProject,
    emptyTrash,
    restoreProject: vi.fn(),
  }) as unknown as ProjectStore;

const renderTrashModal = () =>
  render(() => (
    <ConfigProvider>
      <TrashModal store={fakeStore()} onClose={() => {}} />
    </ConfigProvider>
  ));

describe('TrashModal — el borrado irreversible confirma siempre', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // El flag APAGADO es la condición interesante: aun así tiene que preguntar.
    setUiConfigOverrides({ confirm_delete: false });
    vi.mocked(listTrash).mockResolvedValue(trashItems);
    vi.mocked(purgeProject).mockResolvedValue(undefined);
    vi.mocked(emptyTrash).mockResolvedValue(undefined);
  });

  it('purgar pide confirmación aunque confirm_delete esté en false', async () => {
    vi.mocked(confirm).mockResolvedValue(true);
    const user = userEvent.setup();

    renderTrashModal();
    await screen.findByText('Proyecto borrado');

    await user.click(
      screen.getByRole('button', { name: /eliminar definitivamente/i })
    );

    await waitFor(() => expect(confirm).toHaveBeenCalledTimes(1));
    expect(purgeProject).toHaveBeenCalledWith(7);
  });

  it('vaciar la papelera pide confirmación aunque confirm_delete esté en false', async () => {
    vi.mocked(confirm).mockResolvedValue(true);
    const user = userEvent.setup();

    renderTrashModal();
    await screen.findByText('Proyecto borrado');

    await user.click(screen.getByRole('button', { name: /vaciar papelera/i }));

    await waitFor(() => expect(confirm).toHaveBeenCalledTimes(1));
    expect(emptyTrash).toHaveBeenCalled();
  });

  it('si el usuario cancela, no se borra nada', async () => {
    vi.mocked(confirm).mockResolvedValue(false);
    const user = userEvent.setup();

    renderTrashModal();
    await screen.findByText('Proyecto borrado');

    await user.click(
      screen.getByRole('button', { name: /eliminar definitivamente/i })
    );

    await waitFor(() => expect(confirm).toHaveBeenCalledTimes(1));
    expect(purgeProject).not.toHaveBeenCalled();
  });
});
