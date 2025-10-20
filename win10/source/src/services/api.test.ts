import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import * as api from './api';
import type { CreateProjectDTO, UpdateProjectDTO } from '../types/project';

// Mock Tauri's invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('API Service', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('createProject', () => {
    it('should call create_project command with correct data', async () => {
      const mockProject: CreateProjectDTO = {
        name: 'Test Project',
        description: 'Test Description',
        local_path: '/home/user/test',
        documentation_url: 'https://docs.test.com',
        drive_link: 'https://drive.test.com',
      };

      const mockResponse = {
        id: 1,
        ...mockProject,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      };

      vi.mocked(invoke).mockResolvedValue(mockResponse);

      const result = await api.createProject(mockProject);

      expect(invoke).toHaveBeenCalledWith('create_project', {
        project: mockProject,
      });
      expect(result).toEqual(mockResponse);
    });
  });

  describe('getAllProjects', () => {
    it('should call get_all_projects command', async () => {
      const mockProjects = [
        {
          id: 1,
          name: 'Project 1',
          description: 'Desc 1',
          local_path: '/path1',
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        },
      ];

      vi.mocked(invoke).mockResolvedValue(mockProjects);

      const result = await api.getAllProjects();

      expect(invoke).toHaveBeenCalledWith('get_all_projects');
      expect(result).toEqual(mockProjects);
    });
  });

  describe('getProject', () => {
    it('should call get_project command with id', async () => {
      const mockProject = {
        id: 1,
        name: 'Project 1',
        description: 'Desc 1',
        local_path: '/path1',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      };

      vi.mocked(invoke).mockResolvedValue(mockProject);

      const result = await api.getProject(1);

      expect(invoke).toHaveBeenCalledWith('get_project', { id: 1 });
      expect(result).toEqual(mockProject);
    });
  });

  describe('updateProject', () => {
    it('should call update_project command with id and updates', async () => {
      const updates: UpdateProjectDTO = {
        name: 'Updated Name',
        description: 'Updated Desc',
      };

      const mockResponse = {
        id: 1,
        name: 'Updated Name',
        description: 'Updated Desc',
        local_path: '/path1',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-02T00:00:00Z',
      };

      vi.mocked(invoke).mockResolvedValue(mockResponse);

      const result = await api.updateProject(1, updates);

      expect(invoke).toHaveBeenCalledWith('update_project', {
        id: 1,
        updates,
      });
      expect(result).toEqual(mockResponse);
    });
  });

  describe('deleteProject', () => {
    it('should call delete_project command with id', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);

      await api.deleteProject(1);

      expect(invoke).toHaveBeenCalledWith('delete_project', { id: 1 });
    });
  });

  describe('searchProjects', () => {
    it('should call search_projects command with query', async () => {
      const mockProjects = [
        {
          id: 1,
          name: 'Test Project',
          description: 'Test Desc',
          local_path: '/test',
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        },
      ];

      vi.mocked(invoke).mockResolvedValue(mockProjects);

      const result = await api.searchProjects('test');

      expect(invoke).toHaveBeenCalledWith('search_projects', { query: 'test' });
      expect(result).toEqual(mockProjects);
    });
  });

  describe('openTerminal', () => {
    it('should call open_terminal command with path', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);

      await api.openTerminal('/home/user/project');

      expect(invoke).toHaveBeenCalledWith('open_terminal', {
        path: '/home/user/project',
      });
    });
  });

  // ==================== TODOS API TESTS ====================

  describe('TODOs API', () => {
    describe('createTodo', () => {
      it('should create a TODO', async () => {
        const mockTodo = {
          project_id: 1,
          content: 'Implementar feature X',
        };

        const mockResponse = {
          id: 1,
          ...mockTodo,
          is_completed: false,
          created_at: '2024-01-01T00:00:00Z',
          completed_at: null,
        };

        vi.mocked(invoke).mockResolvedValue(mockResponse);

        const result = await api.createTodo(mockTodo);

        expect(invoke).toHaveBeenCalledWith('create_todo', { todo: mockTodo });
        expect(result).toEqual(mockResponse);
        expect(result.is_completed).toBe(false);
      });
    });

    describe('getProjectTodos', () => {
      it('should get all TODOs for a project', async () => {
        const mockTodos = [
          {
            id: 1,
            project_id: 1,
            content: 'TODO 1',
            is_completed: false,
            created_at: '2024-01-01T00:00:00Z',
            completed_at: null,
          },
          {
            id: 2,
            project_id: 1,
            content: 'TODO 2',
            is_completed: true,
            created_at: '2024-01-02T00:00:00Z',
            completed_at: '2024-01-03T00:00:00Z',
          },
        ];

        vi.mocked(invoke).mockResolvedValue(mockTodos);

        const result = await api.getProjectTodos(1);

        expect(invoke).toHaveBeenCalledWith('get_project_todos', {
          projectId: 1,
        });
        expect(result).toHaveLength(2);
        expect(result[0].is_completed).toBe(false);
        expect(result[1].is_completed).toBe(true);
      });
    });

    describe('updateTodo', () => {
      it('should update TODO content', async () => {
        const updates = { content: 'Updated content' };
        const mockResponse = {
          id: 1,
          project_id: 1,
          content: 'Updated content',
          is_completed: false,
          created_at: '2024-01-01T00:00:00Z',
          completed_at: null,
        };

        vi.mocked(invoke).mockResolvedValue(mockResponse);

        const result = await api.updateTodo(1, updates);

        expect(invoke).toHaveBeenCalledWith('update_todo', {
          id: 1,
          updates,
        });
        expect(result.content).toBe('Updated content');
      });

      it('should toggle TODO completion', async () => {
        const updates = { is_completed: true };
        const mockResponse = {
          id: 1,
          project_id: 1,
          content: 'Some TODO',
          is_completed: true,
          created_at: '2024-01-01T00:00:00Z',
          completed_at: '2024-01-05T00:00:00Z',
        };

        vi.mocked(invoke).mockResolvedValue(mockResponse);

        const result = await api.updateTodo(1, updates);

        expect(result.is_completed).toBe(true);
        expect(result.completed_at).toBeTruthy();
      });
    });

    describe('deleteTodo', () => {
      it('should delete a TODO', async () => {
        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.deleteTodo(1);

        expect(invoke).toHaveBeenCalledWith('delete_todo', { id: 1 });
      });
    });
  });

  // ==================== STATUS & PIN API TESTS ====================

  describe('Status & Pin API', () => {
    describe('updateProjectStatus', () => {
      it('should update project status to "pausado"', async () => {
        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.updateProjectStatus(1, 'pausado');

        expect(invoke).toHaveBeenCalledWith('update_project_status', {
          projectId: 1,
          status: 'pausado',
        });
      });

      it('should update project status to "completado"', async () => {
        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.updateProjectStatus(1, 'completado');

        expect(invoke).toHaveBeenCalledWith('update_project_status', {
          projectId: 1,
          status: 'completado',
        });
      });
    });

    describe('togglePinProject', () => {
      it('should pin a project and return true', async () => {
        vi.mocked(invoke).mockResolvedValue(true);

        const result = await api.togglePinProject(1);

        expect(invoke).toHaveBeenCalledWith('toggle_pin_project', {
          projectId: 1,
        });
        expect(result).toBe(true);
      });

      it('should unpin a project and return false', async () => {
        vi.mocked(invoke).mockResolvedValue(false);

        const result = await api.togglePinProject(1);

        expect(result).toBe(false);
      });
    });

    describe('reorderPinnedProjects', () => {
      it('should reorder pinned projects', async () => {
        const newOrder = [3, 1, 5, 2];

        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.reorderPinnedProjects(newOrder);

        expect(invoke).toHaveBeenCalledWith('reorder_pinned_projects', {
          projectIds: newOrder,
        });
      });
    });
  });
});
