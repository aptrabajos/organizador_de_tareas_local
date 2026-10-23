import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import userEvent from '@testing-library/user-event';
import { invoke } from '@tauri-apps/api/core';
import toast from 'solid-toast';
import ProjectList from './ProjectList';
import type { Project } from '../types/project';

// "Trabajar" y "Parar" desde la tarjeta del proyecto (P7, P9), con la sesión en la
// base (R8-3): `trabajar` abre la terminal, `parar` devuelve un `ResultadoParar` y la
// tarjeta lee las sesiones vivas con `estado_trabajar`.

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
  vi.mocked(toast).mockClear();
  vi.mocked(toast.error).mockClear();
  vi.mocked(toast.success).mockClear();
});

afterEach(() => {
  vi.mocked(invoke).mockImplementation(base as typeof invoke);
});

const renderLista = (
  projects: Project[] = [proyecto],
  onProjectsChanged = vi.fn()
) =>
  render(() => (
    <ProjectList
      projects={projects}
      onEdit={vi.fn()}
      onDelete={vi.fn()}
      onProjectsChanged={onProjectsChanged}
    />
  ));

const TOKEN = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

/** Una sesión viva de la app en `projectId`, como la devuelve `estado_trabajar`. */
const sesionViva = (projectId: number) => ({
  token: TOKEN,
  project_id: projectId,
  estado: 'viva_activa',
  dueno: 'app',
  origen: 'boton',
  creada_at: '2026-10-20 09:00:00',
  last_seen_at: null,
  tramo_abierto: null,
  segundos: 0,
  elapsed_seconds: 42,
});

const estado = (sesiones: unknown[]) => ({
  sesiones,
  demonio_vivo: false,
  adopcion_imposible: true,
  tracker_corrio: false,
  sondeos_sin_demonio: 1,
  aviso: { alarma: false, linea_trabajar: sesiones.length > 0 },
});

describe('ProjectList: Trabajar', () => {
  it('muestra un toast en español si trabajar falla y no abre nada más', async () => {
    const user = userEvent.setup();
    pisarComando('trabajar', () =>
      Promise.reject(
        'Proyecto 1 está en la papelera: restauralo para trabajar en él.'
      )
    );
    renderLista();

    await user.click(screen.getByRole('button', { name: /trabajar/i }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledTimes(1);
    });
    const mensaje = vi.mocked(toast.error).mock.calls[0][0] as string;
    expect(mensaje).toContain('No se pudo empezar a trabajar en Proyecto 1');
    // El texto del backend llega tal cual (Tauri rechaza con un string plano).
    expect(mensaje).toContain('está en la papelera');
    expect(invoke).not.toHaveBeenCalledWith('open_terminal', expect.anything());
  });

  it('no muestra error si la sesión arranca bien: la terminal la abre trabajar', async () => {
    const user = userEvent.setup();
    renderLista();

    await user.click(screen.getByRole('button', { name: /trabajar/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('trabajar', { projectId: 1 });
    });
    expect(toast.error).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith('open_terminal', expect.anything());
  });

  it('si falla el registro de la apertura, la sesión arranca igual', async () => {
    const user = userEvent.setup();
    pisarComando('track_project_open', () => Promise.reject('boom'));
    renderLista();

    await user.click(screen.getByRole('button', { name: /trabajar/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('trabajar', { projectId: 1 });
    });
    expect(toast.error).not.toHaveBeenCalled();
  });

  it('con aviso_sin_tracker, avisa que la sesión no se pausa y se corta a las 12 h', async () => {
    const user = userEvent.setup();
    pisarComando('trabajar', () =>
      Promise.resolve({
        token: TOKEN,
        project_id: 1,
        project_name: 'Proyecto 1',
        dueno: 'app',
        sesion_anterior: false,
        tracking_initialized: false,
        aviso_sin_tracker: true,
      })
    );
    renderLista();

    await user.click(screen.getByRole('button', { name: /trabajar/i }));

    await waitFor(() => {
      expect(toast).toHaveBeenCalledWith(
        expect.stringContaining('El tracker no está corriendo'),
        expect.anything()
      );
    });
  });

  it('sin aviso_sin_tracker (adopción imposible) no hay toast del tracker', async () => {
    const user = userEvent.setup();
    renderLista();

    await user.click(screen.getByRole('button', { name: /trabajar/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('trabajar', { projectId: 1 });
    });
    expect(toast).not.toHaveBeenCalledWith(
      expect.stringContaining('El tracker no está corriendo'),
      expect.anything()
    );
  });
});

describe('ProjectList: Parar', () => {
  const otro: Project = { ...proyecto, id: 2, name: 'Proyecto 2' };

  const sesionActivaEn = (projectId: number) =>
    pisarComando('estado_trabajar', () =>
      Promise.resolve(estado([sesionViva(projectId)]))
    );

  it('sin sesión viva no hay botón Parar', async () => {
    renderLista([proyecto, otro]);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('estado_trabajar', {
        sondeosSinDemonio: null,
      });
    });
    expect(screen.queryByRole('button', { name: /parar/i })).toBeNull();
  });

  it('con sesión viva, Parar aparece solo en la tarjeta de ese proyecto', async () => {
    sesionActivaEn(2);
    renderLista([proyecto, otro]);

    const parar = await screen.findByRole('button', { name: /parar/i });
    expect(screen.getAllByRole('button', { name: /parar/i })).toHaveLength(1);
    // El botón vive al lado del "Trabajar" del proyecto 2, no del 1.
    const trabajar = screen.getAllByRole('button', { name: /trabajar/i });
    expect(parar.parentElement).toBe(trabajar[1].parentElement);
    expect(parar.parentElement).not.toBe(trabajar[0].parentElement);
  });

  it('Trabajar hace aparecer Parar, y Parar llama a parar con el token y lo oculta', async () => {
    const user = userEvent.setup();
    let vivas: unknown[] = [];
    pisarComando('estado_trabajar', () => Promise.resolve(estado(vivas)));
    pisarComando('trabajar', () => {
      vivas = [sesionViva(1)];
      return Promise.resolve({
        token: TOKEN,
        project_id: 1,
        project_name: 'Proyecto 1',
        dueno: 'app',
        sesion_anterior: false,
        tracking_initialized: false,
        aviso_sin_tracker: false,
      });
    });
    pisarComando('parar', () => {
      vivas = [];
      return Promise.resolve({ resultado: 'cerrada', segundos: 125 });
    });
    renderLista();

    await user.click(screen.getByRole('button', { name: /trabajar/i }));
    const parar = await screen.findByRole('button', { name: /parar/i });

    await user.click(parar);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('parar', {
        projectId: 1,
        token: TOKEN,
      });
    });
    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /parar/i })).toBeNull();
    });
    expect(toast.success).toHaveBeenCalledWith(
      expect.stringContaining('guardada: 2m'),
      expect.anything()
    );
  });

  it('Parar con tiempo guardado recarga la lista y muestra minutos y segundos', async () => {
    const user = userEvent.setup();
    const onProjectsChanged = vi.fn();
    sesionActivaEn(1);
    pisarComando('parar', () =>
      Promise.resolve({ resultado: 'cerrada', segundos: 417 })
    );
    renderLista([proyecto], onProjectsChanged);

    await user.click(await screen.findByRole('button', { name: /parar/i }));

    await waitFor(() => {
      expect(onProjectsChanged).toHaveBeenCalledTimes(1);
    });
    expect(toast.success).toHaveBeenCalledWith(
      expect.stringContaining('guardada: 6m 57s'),
      expect.anything()
    );
  });

  it('Parar sin tiempo guardado no recarga la lista', async () => {
    const user = userEvent.setup();
    const onProjectsChanged = vi.fn();
    sesionActivaEn(1);
    pisarComando('parar', () =>
      Promise.resolve({ resultado: 'cerrada', segundos: 0 })
    );
    renderLista([proyecto], onProjectsChanged);

    await user.click(await screen.findByRole('button', { name: /parar/i }));

    await waitFor(() => {
      expect(toast.success).toHaveBeenCalled();
    });
    expect(onProjectsChanged).not.toHaveBeenCalled();
  });

  it('Trabajar que cierra la sesión anterior recarga la lista', async () => {
    const user = userEvent.setup();
    const onProjectsChanged = vi.fn();
    pisarComando('trabajar', () =>
      Promise.resolve({
        token: TOKEN,
        project_id: 1,
        project_name: 'Proyecto 1',
        dueno: 'app',
        sesion_anterior: true,
        tracking_initialized: true,
        aviso_sin_tracker: false,
      })
    );
    renderLista([proyecto], onProjectsChanged);

    await user.click(screen.getByRole('button', { name: /trabajar/i }));

    await waitFor(() => {
      expect(onProjectsChanged).toHaveBeenCalledTimes(1);
    });
  });

  it('una sesión de 0 s avisa que no suma tiempo', async () => {
    const user = userEvent.setup();
    sesionActivaEn(1);
    pisarComando('parar', () =>
      Promise.resolve({ resultado: 'cerrada', segundos: 0 })
    );
    renderLista();

    await user.click(await screen.findByRole('button', { name: /parar/i }));

    await waitFor(() => {
      expect(toast.success).toHaveBeenCalledWith(
        expect.stringContaining('no suma tiempo'),
        expect.anything()
      );
    });
  });

  it('Parar sobre una sesión que ya había terminado lo dice con su motivo', async () => {
    const user = userEvent.setup();
    sesionActivaEn(1);
    pisarComando('parar', () =>
      Promise.resolve({
        resultado: 'ya_cerrada',
        motivo: 'terminal_cerrada',
        segundos: 4320,
      })
    );
    renderLista();

    await user.click(await screen.findByRole('button', { name: /parar/i }));

    await waitFor(() => {
      expect(toast).toHaveBeenCalledWith(
        'La sesión de Proyecto 1 ya había terminado: cerraste la terminal (1h 12m).',
        expect.anything()
      );
    });
    expect(toast.success).not.toHaveBeenCalled();
  });

  it('Parar sin sesión que parar no muestra ningún toast', async () => {
    const user = userEvent.setup();
    sesionActivaEn(1);
    pisarComando('parar', () => Promise.resolve({ resultado: 'ninguna' }));
    renderLista();

    await user.click(await screen.findByRole('button', { name: /parar/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('parar', {
        projectId: 1,
        token: TOKEN,
      });
    });
    expect(toast).not.toHaveBeenCalled();
    expect(toast.success).not.toHaveBeenCalled();
    expect(toast.error).not.toHaveBeenCalled();
  });

  it('al ganar el foco, la tarjeta relee las sesiones de la base', async () => {
    renderLista();
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('estado_trabajar', {
        sondeosSinDemonio: null,
      });
    });
    expect(screen.queryByRole('button', { name: /parar/i })).toBeNull();

    // La sesión arrancó en otro lado (el Dashboard, la CLI).
    sesionActivaEn(1);
    window.dispatchEvent(new Event('focus'));

    expect(await screen.findByRole('button', { name: /parar/i })).toBeTruthy();
  });

  it('si Parar falla, muestra el error y el botón sigue', async () => {
    const user = userEvent.setup();
    sesionActivaEn(1);
    pisarComando('parar', () =>
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
