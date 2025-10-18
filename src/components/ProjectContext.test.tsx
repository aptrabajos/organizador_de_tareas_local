import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import ProjectContext from './ProjectContext';
import type {
  Project,
  JournalEntry,
  ProjectTodo,
  ProjectLink,
} from '../types/project';

// Mock de las APIs
vi.mock('../services/api', () => ({
  getProject: vi.fn(),
  getJournalEntries: vi.fn(),
  getProjectTodos: vi.fn(),
  getProjectLinks: vi.fn(),
}));

import {
  getProject,
  getJournalEntries,
  getProjectTodos,
  getProjectLinks,
} from '../services/api';

describe('ProjectContext', () => {
  const mockProject: Project = {
    id: 1,
    name: 'Proyecto Test',
    description: 'Descripción del proyecto de prueba',
    path: '/home/user/proyecto-test',
    tags: 'react,typescript,testing',
    status: 'activo',
    is_pinned: false,
    created_at: '2025-01-01T10:00:00Z',
    updated_at: '2025-01-15T14:30:00Z',
  };

  const mockJournalEntries: JournalEntry[] = [
    {
      id: 1,
      project_id: 1,
      content: 'Última entrada del diario',
      created_at: '2025-01-15T14:00:00Z',
    },
    {
      id: 2,
      project_id: 1,
      content: 'Segunda entrada',
      created_at: '2025-01-14T10:00:00Z',
    },
  ];

  const mockTodos: ProjectTodo[] = [
    {
      id: 1,
      project_id: 1,
      content: 'TODO pendiente 1',
      is_completed: false,
      created_at: '2025-01-10T09:00:00Z',
    },
    {
      id: 2,
      project_id: 1,
      content: 'TODO completado',
      is_completed: true,
      created_at: '2025-01-09T08:00:00Z',
      completed_at: '2025-01-10T12:00:00Z',
    },
  ];

  const mockLinks: ProjectLink[] = [
    {
      id: 1,
      project_id: 1,
      link_type: 'github',
      url: 'https://github.com/user/repo',
      description: 'Repositorio principal',
      created_at: '2025-01-01T10:00:00Z',
    },
  ];

  const mockOnClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    (getProject as ReturnType<typeof vi.fn>).mockResolvedValue(mockProject);
    (getJournalEntries as ReturnType<typeof vi.fn>).mockResolvedValue(
      mockJournalEntries
    );
    (getProjectTodos as ReturnType<typeof vi.fn>).mockResolvedValue(mockTodos);
    (getProjectLinks as ReturnType<typeof vi.fn>).mockResolvedValue(mockLinks);
  });

  describe('Rendering', () => {
    it('should render project context modal with project info', async () => {
      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Proyecto Test')).toBeInTheDocument();
      });

      expect(
        screen.getByText('Descripción del proyecto de prueba')
      ).toBeInTheDocument();
      expect(screen.getByText(/react/)).toBeInTheDocument();
      expect(screen.getByText(/typescript/)).toBeInTheDocument();
      expect(screen.getByText(/activo/i)).toBeInTheDocument();
    });

    it('should render recent journal entries section', async () => {
      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText(/diario reciente/i)).toBeInTheDocument();
      });

      expect(screen.getByText('Última entrada del diario')).toBeInTheDocument();
      expect(screen.getByText('Segunda entrada')).toBeInTheDocument();
    });

    it('should render pending TODOs section', async () => {
      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText(/tareas pendientes/i)).toBeInTheDocument();
      });

      expect(screen.getByText('TODO pendiente 1')).toBeInTheDocument();
      // No debe mostrar TODOs completados
      expect(screen.queryByText('TODO completado')).not.toBeInTheDocument();
    });

    it('should render project links section', async () => {
      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText(/enlaces/i)).toBeInTheDocument();
      });

      expect(screen.getByText('Repositorio principal')).toBeInTheDocument();
      const link = screen.getByRole('link', { name: /repositorio principal/i });
      expect(link).toHaveAttribute('href', 'https://github.com/user/repo');
    });

    it('should show empty state when no journal entries', async () => {
      (getJournalEntries as ReturnType<typeof vi.fn>).mockResolvedValue([]);

      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText(/sin entradas de diario/i)).toBeInTheDocument();
      });
    });

    it('should show empty state when no pending TODOs', async () => {
      (getProjectTodos as ReturnType<typeof vi.fn>).mockResolvedValue([
        { ...mockTodos[1] }, // Solo completados
      ]);

      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText(/sin tareas pendientes/i)).toBeInTheDocument();
      });
    });

    it('should show empty state when no links', async () => {
      (getProjectLinks as ReturnType<typeof vi.fn>).mockResolvedValue([]);

      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText(/sin enlaces/i)).toBeInTheDocument();
      });
    });
  });

  describe('Interactions', () => {
    it('should call onClose when close button is clicked', async () => {
      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Proyecto Test')).toBeInTheDocument();
      });

      const closeButton = screen.getByRole('button', { name: /cerrar/i });
      closeButton.click();

      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });

    it('should call onClose when clicking outside modal', async () => {
      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Proyecto Test')).toBeInTheDocument();
      });

      const backdrop = screen.getByTestId('modal-backdrop');
      backdrop.click();

      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });
  });

  describe('Loading and Error States', () => {
    it('should show loading state while fetching data', () => {
      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      expect(screen.getByRole('status')).toBeInTheDocument();
      expect(screen.getByText(/cargando/i)).toBeInTheDocument();
    });

    it('should show error message when API fails', async () => {
      (getProject as ReturnType<typeof vi.fn>).mockRejectedValue(
        new Error('API Error')
      );

      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText(/error al cargar/i)).toBeInTheDocument();
      });
    });
  });

  describe('Data Formatting', () => {
    it('should limit journal entries to maximum 5 recent', async () => {
      const manyEntries: JournalEntry[] = Array.from(
        { length: 10 },
        (_, i) => ({
          id: i + 1,
          project_id: 1,
          content: `Entrada ${i + 1}`,
          created_at: new Date(2025, 0, 15 - i).toISOString(),
        })
      );

      (getJournalEntries as ReturnType<typeof vi.fn>).mockResolvedValue(
        manyEntries
      );

      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Entrada 1')).toBeInTheDocument();
      });

      // Debe mostrar solo las 5 más recientes
      expect(screen.getByText('Entrada 5')).toBeInTheDocument();
      expect(screen.queryByText('Entrada 6')).not.toBeInTheDocument();
    });

    it('should display tags as individual badges', async () => {
      render(() => <ProjectContext projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Proyecto Test')).toBeInTheDocument();
      });

      // Verificar que cada tag esté en su propio badge
      const reactBadge = screen.getByText('react');
      const tsBadge = screen.getByText('typescript');
      const testBadge = screen.getByText('testing');

      expect(reactBadge).toBeInTheDocument();
      expect(tsBadge).toBeInTheDocument();
      expect(testBadge).toBeInTheDocument();
    });
  });
});
