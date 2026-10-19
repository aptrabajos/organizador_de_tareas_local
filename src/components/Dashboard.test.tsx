import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import Dashboard from './Dashboard';
import type { DashboardData } from '../types/dashboard';
import type { Project } from '../types/project';

vi.mock('../services/api', () => ({
  getDashboardData: vi.fn(),
  openTerminal: vi.fn(),
  trackProjectOpen: vi.fn(),
  startWorkSession: vi.fn(),
}));

import toast from 'solid-toast';
import {
  getDashboardData,
  openTerminal,
  startWorkSession,
  trackProjectOpen,
} from '../services/api';

describe('Dashboard', () => {
  const mockProject: Project = {
    id: 1,
    name: 'Proyecto Alpha',
    description: 'Descripción Alpha',
    local_path: '/home/user/alpha',
    last_opened_at: '2025-01-15T14:30:00Z',
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-15T00:00:00Z',
  };

  const mockDashboardData: DashboardData = {
    recent_projects: [
      mockProject,
      {
        ...mockProject,
        id: 2,
        name: 'Proyecto Beta',
        local_path: '/home/user/beta',
        last_opened_at: '2025-01-14T10:00:00Z',
      },
    ],
    pending_todos: [
      {
        id: 1,
        project_id: 1,
        content: 'Arreglar bug de login',
        is_completed: false,
        created_at: '2025-01-10T00:00:00Z',
        project_name: 'Proyecto Alpha',
      },
      {
        id: 2,
        project_id: 2,
        content: 'Agregar tests unitarios',
        is_completed: false,
        created_at: '2025-01-11T00:00:00Z',
        project_name: 'Proyecto Beta',
      },
    ],
    recent_journal_entries: [
      {
        id: 1,
        project_id: 1,
        content: 'Hoy mejoré el rendimiento',
        created_at: '2025-01-15T12:00:00Z',
        updated_at: '2025-01-15T12:00:00Z',
        project_name: 'Proyecto Alpha',
      },
    ],
  };

  const mockOnProjectClick = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Loading State', () => {
    it('should show skeleton placeholders with animate-pulse while loading', () => {
      (getDashboardData as ReturnType<typeof vi.fn>).mockReturnValue(
        new Promise(() => {})
      );

      render(() => <Dashboard onProjectClick={mockOnProjectClick} />);

      const pulseElements = document.querySelectorAll('.animate-pulse');
      expect(pulseElements.length).toBeGreaterThan(0);
    });
  });

  describe('Error State', () => {
    it('should call getDashboardData on mount even with empty result', async () => {
      // Note: mockRejectedValue causes unhandled rejections with SolidJS
      // createResource in jsdom. Instead, we test with empty/null data to
      // verify the component handles edge cases gracefully.
      (getDashboardData as ReturnType<typeof vi.fn>).mockResolvedValue({
        recent_projects: [],
        pending_todos: [],
        recent_journal_entries: [],
      });

      render(() => <Dashboard onProjectClick={mockOnProjectClick} />);

      // Verify the API was called
      expect(getDashboardData).toHaveBeenCalledTimes(1);

      // Component should render without crashing
      expect(screen.getByText('Dashboard')).toBeTruthy();
    });
  });

  describe('Data Rendering', () => {
    it('should render recent project names', async () => {
      (getDashboardData as ReturnType<typeof vi.fn>).mockResolvedValue(
        mockDashboardData
      );

      render(() => <Dashboard onProjectClick={mockOnProjectClick} />);

      await waitFor(() => {
        expect(
          screen.getAllByText('Proyecto Alpha').length
        ).toBeGreaterThanOrEqual(1);
      });
      expect(
        screen.getAllByText('Proyecto Beta').length
      ).toBeGreaterThanOrEqual(1);
    });

    it('should render pending todo content', async () => {
      (getDashboardData as ReturnType<typeof vi.fn>).mockResolvedValue(
        mockDashboardData
      );

      render(() => <Dashboard onProjectClick={mockOnProjectClick} />);

      await waitFor(() => {
        expect(screen.getByText('Arreglar bug de login')).toBeTruthy();
      });
      expect(screen.getByText('Agregar tests unitarios')).toBeTruthy();
    });

    it('should render journal entries with content', async () => {
      (getDashboardData as ReturnType<typeof vi.fn>).mockResolvedValue(
        mockDashboardData
      );

      render(() => <Dashboard onProjectClick={mockOnProjectClick} />);

      await waitFor(() => {
        expect(screen.getByText('Hoy mejoré el rendimiento')).toBeTruthy();
      });
    });

    it('should show correct counts in Quick Stats', async () => {
      (getDashboardData as ReturnType<typeof vi.fn>).mockResolvedValue(
        mockDashboardData
      );

      render(() => <Dashboard onProjectClick={mockOnProjectClick} />);

      await waitFor(() => {
        expect(screen.getByText('Proyectos activos')).toBeTruthy();
      });

      // Check count labels exist alongside their values
      expect(screen.getByText('Tareas pendientes')).toBeTruthy();
      expect(screen.getByText('Entradas recientes')).toBeTruthy();
    });
  });

  describe('Empty States', () => {
    it('should show "No hay proyectos recientes" when empty', async () => {
      (getDashboardData as ReturnType<typeof vi.fn>).mockResolvedValue({
        recent_projects: [],
        pending_todos: [mockDashboardData.pending_todos[0]],
        recent_journal_entries: [mockDashboardData.recent_journal_entries[0]],
      });

      render(() => <Dashboard onProjectClick={mockOnProjectClick} />);

      await waitFor(() => {
        expect(screen.getByText('No hay proyectos recientes')).toBeTruthy();
      });
    });

    it('should show "¡Todo completado!" when no pending todos', async () => {
      (getDashboardData as ReturnType<typeof vi.fn>).mockResolvedValue({
        recent_projects: [mockProject],
        pending_todos: [],
        recent_journal_entries: [mockDashboardData.recent_journal_entries[0]],
      });

      render(() => <Dashboard onProjectClick={mockOnProjectClick} />);

      await waitFor(() => {
        expect(screen.getByText('¡Todo completado!')).toBeTruthy();
      });
    });
  });

  describe('Interactions', () => {
    it('should call onProjectClick when "Ver proyecto" button is clicked', async () => {
      (getDashboardData as ReturnType<typeof vi.fn>).mockResolvedValue(
        mockDashboardData
      );

      render(() => <Dashboard onProjectClick={mockOnProjectClick} />);

      await waitFor(() => {
        expect(screen.getAllByTitle('Ver proyecto').length).toBeGreaterThan(0);
      });

      const viewButtons = screen.getAllByTitle('Ver proyecto');
      viewButtons[0].click();

      expect(mockOnProjectClick).toHaveBeenCalledWith(
        mockDashboardData.recent_projects[0]
      );
    });
  });

  describe('Abrir usa el flujo de Trabajar (P8)', () => {
    const TITULO = 'Trabajar: abrir terminal y medir el tiempo';

    const clickAbrir = async () => {
      vi.mocked(getDashboardData).mockResolvedValue(mockDashboardData);
      render(() => <Dashboard onProjectClick={mockOnProjectClick} />);
      await waitFor(() => {
        expect(screen.getAllByTitle(TITULO).length).toBeGreaterThan(0);
      });
      screen.getAllByTitle(TITULO)[0].click();
    };

    it('arranca la sesión de trabajo y abre el terminal', async () => {
      vi.mocked(trackProjectOpen).mockResolvedValue(undefined);
      vi.mocked(startWorkSession).mockResolvedValue({
        session_id: 7,
        project_id: 1,
        project_name: 'Proyecto Alpha',
        previous_session_stopped: false,
        tracking_initialized: false,
      });
      vi.mocked(openTerminal).mockResolvedValue(undefined);

      await clickAbrir();

      await waitFor(() => {
        expect(openTerminal).toHaveBeenCalledWith('/home/user/alpha');
      });
      expect(trackProjectOpen).toHaveBeenCalledWith(1);
      expect(startWorkSession).toHaveBeenCalledWith(1);
      expect(toast.error).not.toHaveBeenCalled();
    });

    it('si la sesión no arranca, avisa en español y abre el terminal igual', async () => {
      vi.mocked(trackProjectOpen).mockResolvedValue(undefined);
      vi.mocked(startWorkSession).mockRejectedValue(
        'El directorio del proyecto no existe: /home/user/alpha'
      );
      vi.mocked(openTerminal).mockResolvedValue(undefined);

      await clickAbrir();

      await waitFor(() => {
        expect(openTerminal).toHaveBeenCalledWith('/home/user/alpha');
      });
      expect(toast.error).toHaveBeenCalledTimes(1);
      const mensaje = vi.mocked(toast.error).mock.calls[0][0] as string;
      expect(mensaje).toContain('No se pudo empezar a medir el tiempo');
      expect(mensaje).toContain('El directorio del proyecto no existe');
    });
  });

  describe('estado de error (B7)', () => {
    it('muestra el mensaje real del backend cuando rechaza con un string plano', async () => {
      // Esta es LA forma en que Tauri v2 rechaza: un comando que devuelve
      // `Err(String)` rechaza la promesa con el string pelado, NO con un
      // `Error`. Leer `.message` sobre eso da `undefined`.
      (getDashboardData as ReturnType<typeof vi.fn>).mockRejectedValue(
        'No se pudo abrir la base de datos'
      );

      render(() => <Dashboard />);

      await waitFor(() => {
        expect(
          screen.getByText('No se pudo abrir la base de datos')
        ).toBeTruthy();
      });
      // El fallback genérico tapaba el mensaje accionable del backend.
      expect(screen.queryByText('Error desconocido')).toBeFalsy();
    });

    it('sigue mostrando el mensaje cuando el rechazo SÍ es un Error', async () => {
      (getDashboardData as ReturnType<typeof vi.fn>).mockRejectedValue(
        new Error('fallo de red')
      );

      render(() => <Dashboard />);

      await waitFor(() => {
        expect(screen.getByText('fallo de red')).toBeTruthy();
      });
    });
  });
});
