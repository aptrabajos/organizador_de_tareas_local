import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import userEvent from '@testing-library/user-event';
import TodoList from './TodoList';
import type { ProjectTodo } from '../types/project';
import * as api from '../services/api';

// Mock API
vi.mock('../services/api', () => ({
  getProjectTodos: vi.fn(),
  createTodo: vi.fn(),
  updateTodo: vi.fn(),
  deleteTodo: vi.fn(),
}));

describe('TodoList', () => {
  const mockTodos: ProjectTodo[] = [
    {
      id: 1,
      project_id: 1,
      content: 'Implementar feature X',
      is_completed: false,
      created_at: '2024-01-01T00:00:00Z',
    },
    {
      id: 2,
      project_id: 1,
      content: 'Escribir tests',
      is_completed: false,
      created_at: '2024-01-02T00:00:00Z',
    },
    {
      id: 3,
      project_id: 1,
      content: 'Review PR',
      is_completed: true,
      created_at: '2024-01-03T00:00:00Z',
      completed_at: '2024-01-04T00:00:00Z',
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==================== RENDERING TESTS ====================

  describe('Rendering', () => {
    it('should render empty state when no TODOs', async () => {
      vi.mocked(api.getProjectTodos).mockResolvedValue([]);

      render(() => <TodoList projectId={1} />);

      await waitFor(() => {
        expect(
          screen.queryByText(/no hay tareas/i) ||
            screen.queryByText(/sin tareas/i) ||
            screen.queryByText(/agrega tu primera tarea/i)
        ).toBeTruthy();
      });
    });

    it('should render list of TODOs', async () => {
      vi.mocked(api.getProjectTodos).mockResolvedValue(mockTodos);

      render(() => <TodoList projectId={1} />);

      await waitFor(() => {
        expect(screen.getByText('Implementar feature X')).toBeTruthy();
        expect(screen.getByText('Escribir tests')).toBeTruthy();
        expect(screen.getByText('Review PR')).toBeTruthy();
      });
    });

    it('should separate completed from pending', async () => {
      vi.mocked(api.getProjectTodos).mockResolvedValue(mockTodos);

      render(() => <TodoList projectId={1} />);

      await waitFor(() => {
        // Verificar que hay secciones separadas
        const pendingSection = screen.queryByText(/pendientes/i);
        const completedSection = screen.queryByText(/completad/i);

        // Al menos una debe existir para separar visualmente
        expect(pendingSection || completedSection).toBeTruthy();
      });
    });
  });

  // ==================== INTERACTION TESTS ====================

  describe('Interactions', () => {
    it('should create new TODO on button click', async () => {
      const user = userEvent.setup();
      const newTodo = {
        id: 4,
        project_id: 1,
        content: 'Nueva tarea',
        is_completed: false,
        created_at: new Date().toISOString(),
      };

      vi.mocked(api.getProjectTodos).mockResolvedValue([]);
      vi.mocked(api.createTodo).mockResolvedValue(newTodo);

      render(() => <TodoList projectId={1} />);

      // Buscar input y botón
      const input = (await screen.findByPlaceholderText(
        /nueva tarea/i
      )) as HTMLInputElement;
      const button = screen.getByRole('button', { name: /agregar/i });

      // Escribir y crear
      await user.type(input, 'Nueva tarea');
      await user.click(button);

      await waitFor(() => {
        expect(api.createTodo).toHaveBeenCalledWith({
          project_id: 1,
          content: 'Nueva tarea',
        });
      });
    });

    it('should not create empty TODOs', async () => {
      const user = userEvent.setup();

      vi.mocked(api.getProjectTodos).mockResolvedValue([]);

      render(() => <TodoList projectId={1} />);

      // Esperar a que termine de cargar
      await waitFor(() => {
        expect(screen.queryByRole('status')).toBeFalsy();
      });

      const button = screen.getByRole('button', { name: /agregar/i });

      // Intentar crear sin escribir nada
      await user.click(button);

      // No debe llamar a la API
      expect(api.createTodo).not.toHaveBeenCalled();
    });

    it('should toggle TODO completion on checkbox click', async () => {
      const user = userEvent.setup();

      vi.mocked(api.getProjectTodos).mockResolvedValue(mockTodos);
      vi.mocked(api.updateTodo).mockResolvedValue({
        ...mockTodos[0],
        is_completed: true,
        completed_at: new Date().toISOString(),
      });

      render(() => <TodoList projectId={1} />);

      await waitFor(() => {
        expect(screen.getByText('Implementar feature X')).toBeTruthy();
      });

      // Buscar checkbox del primer TODO
      const checkboxes = screen.getAllByRole('checkbox');
      await user.click(checkboxes[0]);

      await waitFor(() => {
        expect(api.updateTodo).toHaveBeenCalledWith(1, { is_completed: true });
      });
    });

    it('should delete TODO on delete button click', async () => {
      const user = userEvent.setup();

      vi.mocked(api.getProjectTodos).mockResolvedValue(mockTodos);
      vi.mocked(api.deleteTodo).mockResolvedValue(undefined);

      // Mock window.confirm
      const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);

      render(() => <TodoList projectId={1} />);

      await waitFor(() => {
        expect(screen.getByText('Implementar feature X')).toBeTruthy();
      });

      // Buscar botón de eliminar (🗑️)
      const deleteButtons = screen.getAllByRole('button', {
        name: /eliminar|delete|🗑/i,
      });

      await user.click(deleteButtons[0]);

      await waitFor(() => {
        expect(api.deleteTodo).toHaveBeenCalledWith(1);
      });

      confirmSpy.mockRestore();
    });
  });

  // ==================== EDGE CASES ====================

  describe('Edge Cases', () => {
    it('should show loading state while fetching', () => {
      vi.mocked(api.getProjectTodos).mockImplementation(
        () => new Promise(() => {}) // Never resolves
      );

      render(() => <TodoList projectId={1} />);

      // Debe mostrar algún indicador de carga
      const loadingIndicator =
        screen.queryByText(/cargando/i) || screen.queryByRole('status');

      expect(loadingIndicator).toBeTruthy();
    });

    it('should show error message on API failure', async () => {
      vi.mocked(api.getProjectTodos).mockRejectedValue(
        new Error('Network error')
      );

      render(() => <TodoList projectId={1} />);

      await waitFor(() => {
        const errorMessage = screen.queryByText(/error/i);
        expect(errorMessage).toBeTruthy();
      });
    });
  });
});
