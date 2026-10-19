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
  vi.mocked(invoke).mockClear();
  vi.mocked(toast.error).mockClear();
  vi.mocked(toast.success).mockClear();
});

afterEach(() => {
  vi.mocked(invoke).mockImplementation(base as typeof invoke);
});

const renderLista = (
  projects: Project[] = [proyecto],
  onOpenTerminal = vi.fn()
) =>
  render(() => (
    <ProjectList
      projects={projects}
      onEdit={vi.fn()}
      onDelete={vi.fn()}
      onOpenTerminal={onOpenTerminal}
    />
  ));

describe('ProjectList: Trabajar', () => {
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

describe('ProjectList: Parar', () => {
  const otro: Project = { ...proyecto, id: 2, name: 'Proyecto 2' };

  const sesionActivaEn = (projectId: number) =>
    pisarComando('get_work_session_status', () =>
      Promise.resolve({
        is_tracking: true,
        project_id: projectId,
        project_path: '/home/user/proyecto1',
        elapsed_seconds: 42,
      })
    );

  it('sin sesión activa no hay botón Parar', async () => {
    renderLista([proyecto, otro]);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_work_session_status');
    });
    expect(screen.queryByRole('button', { name: /parar/i })).toBeNull();
  });

  it('con sesión activa, Parar aparece solo en la tarjeta de ese proyecto', async () => {
    sesionActivaEn(2);
    renderLista([proyecto, otro]);

    const parar = await screen.findByRole('button', { name: /parar/i });
    expect(screen.getAllByRole('button', { name: /parar/i })).toHaveLength(1);
    // El botón vive al lado del "Trabajar" del proyecto 2, no del 1.
    const trabajar = screen.getAllByRole('button', { name: /trabajar/i });
    expect(parar.parentElement).toBe(trabajar[1].parentElement);
    expect(parar.parentElement).not.toBe(trabajar[0].parentElement);
  });

  it('Trabajar hace aparecer Parar, y Parar llama a stop_work_session y lo oculta', async () => {
    const user = userEvent.setup();
    pisarComando('stop_work_session', () => Promise.resolve(125));
    renderLista();

    await user.click(screen.getByRole('button', { name: /trabajar/i }));
    const parar = await screen.findByRole('button', { name: /parar/i });

    await user.click(parar);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('stop_work_session');
    });
    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /parar/i })).toBeNull();
    });
    expect(toast.success).toHaveBeenCalledWith(
      expect.stringContaining('guardada: 2m'),
      expect.anything()
    );
  });

  it('una sesión de 0 s avisa que no suma tiempo', async () => {
    const user = userEvent.setup();
    sesionActivaEn(1);
    pisarComando('stop_work_session', () => Promise.resolve(0));
    renderLista();

    await user.click(await screen.findByRole('button', { name: /parar/i }));

    await waitFor(() => {
      expect(toast.success).toHaveBeenCalledWith(
        expect.stringContaining('no suma tiempo'),
        expect.anything()
      );
    });
  });

  it('si Parar falla, muestra el error y el botón sigue', async () => {
    const user = userEvent.setup();
    sesionActivaEn(1);
    pisarComando('stop_work_session', () =>
      Promise.reject('Error stopping session: disk I/O error')
    );
    renderLista();

    await user.click(await screen.findByRole('button', { name: /parar/i }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledTimes(1);
    });
    const mensaje = vi.mocked(toast.error).mock.calls[0][0] as string;
    expect(mensaje).toContain('No se pudo parar la sesión');
    expect(mensaje).toContain('disk I/O error');
    expect(screen.getByRole('button', { name: /parar/i })).toBeTruthy();
  });
});
