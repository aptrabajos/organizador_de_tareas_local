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
});
