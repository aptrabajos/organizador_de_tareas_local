import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import userEvent from '@testing-library/user-event';
import { invoke } from '@tauri-apps/api/core';
import toast from 'solid-toast';
import ProjectList from './ProjectList';
import type { Project } from '../types/project';

// "Trabajar" y "Parar" desde la tarjeta del proyecto (P7, P9).

const proyecto: Project = {
  id: 1,
  name: 'Proyecto 1',
  description: 'Descripción del proyecto 1',
  local_path: '/home/user/proyecto1',
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
};

type InvokeImpl = (cmd: string, args?: Record<string, unknown>) => unknown;

describe('ProjectList: Trabajar', () => {
  // Mock base de test-setup.ts; se restaura después de cada test.
  let base: InvokeImpl;

  /** Pisa un comando del mock de invoke y deja pasar el resto al mock actual. */
  const pisarComando = (comando: string, respuesta: () => Promise<unknown>) => {
    const actual = vi.mocked(invoke).getMockImplementation() as InvokeImpl;
    vi.mocked(invoke).mockImplementation(((
      cmd: string,
      args?: Record<string, unknown>
    ) => (cmd === comando ? respuesta() : actual(cmd, args))) as typeof invoke);
  };

  beforeEach(() => {
    base = vi.mocked(invoke).getMockImplementation() as InvokeImpl;
    vi.mocked(toast.error).mockClear();
    vi.mocked(toast.success).mockClear();
  });

  afterEach(() => {
    vi.mocked(invoke).mockImplementation(base as typeof invoke);
  });

  it('muestra un toast en español si startWorkSession falla y abre el terminal igual', async () => {
    const user = userEvent.setup();
    const onOpenTerminal = vi.fn();
    pisarComando('start_work_session', () =>
      Promise.reject('El directorio del proyecto no existe: /home/user/x')
    );

    render(() => (
      <ProjectList
        projects={[proyecto]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onOpenTerminal={onOpenTerminal}
      />
    ));

    await user.click(screen.getByRole('button', { name: /trabajar/i }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledTimes(1);
    });
    const mensaje = vi.mocked(toast.error).mock.calls[0][0] as string;
    expect(mensaje).toContain('No se pudo empezar a medir el tiempo');
    // El texto del backend llega tal cual (Tauri rechaza con un string plano).
    expect(mensaje).toContain(
      'El directorio del proyecto no existe: /home/user/x'
    );
    expect(onOpenTerminal).toHaveBeenCalledTimes(1);
    expect(onOpenTerminal).toHaveBeenCalledWith(proyecto);
  });

  it('no muestra error si la sesión arranca bien', async () => {
    const user = userEvent.setup();
    const onOpenTerminal = vi.fn();

    render(() => (
      <ProjectList
        projects={[proyecto]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onOpenTerminal={onOpenTerminal}
      />
    ));

    await user.click(screen.getByRole('button', { name: /trabajar/i }));

    await waitFor(() => {
      expect(onOpenTerminal).toHaveBeenCalledWith(proyecto);
    });
    expect(toast.error).not.toHaveBeenCalled();
  });

  it('si falla el registro de la apertura, la sesión arranca igual', async () => {
    const user = userEvent.setup();
    pisarComando('track_project_open', () => Promise.reject('boom'));

    render(() => (
      <ProjectList
        projects={[proyecto]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onOpenTerminal={vi.fn()}
      />
    ));

    await user.click(screen.getByRole('button', { name: /trabajar/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('start_work_session', {
        projectId: 1,
      });
    });
    expect(toast.error).not.toHaveBeenCalled();
  });
});
