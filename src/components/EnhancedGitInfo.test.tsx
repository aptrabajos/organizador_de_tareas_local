import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import EnhancedGitInfo from './EnhancedGitInfo';
import type { GitFileCount } from '../types/git';

vi.mock('../services/api', () => ({
  getGitBranch: vi.fn(),
  getGitFileCount: vi.fn(),
  getGitModifiedFiles: vi.fn(),
  getRecentCommits: vi.fn(),
  getGitRemoteUrl: vi.fn(),
  getGitAheadBehind: vi.fn(),
  gitAdd: vi.fn(),
  gitPush: vi.fn(),
  gitPull: vi.fn(),
}));

import {
  getGitBranch,
  getGitFileCount,
  getRecentCommits,
  getGitRemoteUrl,
  getGitAheadBehind,
} from '../services/api';

describe('EnhancedGitInfo', () => {
  // Valores DISTINTOS entre sí: si el componente confunde dos campos, el test falla.
  // Con todos en el mismo número un cruce de campos pasaría igual.
  const fileCount: GitFileCount = { modified: 2, staged: 1, untracked: 3 };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getGitBranch).mockResolvedValue('main');
    vi.mocked(getGitFileCount).mockResolvedValue(fileCount);
    vi.mocked(getRecentCommits).mockResolvedValue([]);
    vi.mocked(getGitRemoteUrl).mockResolvedValue(null);
    vi.mocked(getGitAheadBehind).mockResolvedValue([0, 0]);
  });

  it('muestra la rama actual', async () => {
    render(() => <EnhancedGitInfo projectPath="/home/user/proyecto" />);

    expect(await screen.findByText('main')).toBeTruthy();
    expect(getGitBranch).toHaveBeenCalledWith('/home/user/proyecto');
  });

  it('muestra los contadores de staged y modified con sus valores propios', async () => {
    const { container } = render(() => (
      <EnhancedGitInfo projectPath="/home/user/proyecto" />
    ));

    await screen.findByText('main');

    await waitFor(() => {
      // El badge concatena los dos contadores en un solo span: "1 staged, 2 modified".
      expect(container.textContent).toContain('1 staged');
      expect(container.textContent).toContain('2 modified');
    });
  });

  it('NO muestra el contador de untracked (hueco conocido de la UI)', async () => {
    // El contrato `GitFileCount` y el backend devuelven `untracked`, pero el badge
    // solo pinta `staged` y `modified`. Este test fija el comportamiento REAL de hoy:
    // si alguien agrega untracked al badge, va a fallar y hay que actualizarlo acá.
    const { container } = render(() => (
      <EnhancedGitInfo projectPath="/home/user/proyecto" />
    ));

    await screen.findByText('main');
    await waitFor(() => expect(container.textContent).toContain('1 staged'));

    expect(container.textContent).not.toContain('untracked');
    expect(container.textContent).not.toContain('3');
  });

  it('oculta el badge cuando solo hay archivos untracked', async () => {
    // Consecuencia directa del hueco anterior: la condición de visibilidad es
    // `modified + staged > 0`, así que un repo con SOLO archivos nuevos se ve limpio.
    vi.mocked(getGitFileCount).mockResolvedValue({
      modified: 0,
      staged: 0,
      untracked: 3,
    });

    const { container } = render(() => (
      <EnhancedGitInfo projectPath="/home/user/proyecto" />
    ));

    await screen.findByText('main');

    expect(container.textContent).not.toContain('staged');
    expect(container.textContent).not.toContain('modified');
  });

  it('no renderiza nada si el path no es un repo git', async () => {
    vi.mocked(getGitBranch).mockRejectedValue('no es un repositorio git');

    const { container } = render(() => (
      <EnhancedGitInfo projectPath="/home/user/sin-git" />
    ));

    await waitFor(() => expect(getGitBranch).toHaveBeenCalled());
    expect(container.textContent).toBe('');
  });
});
