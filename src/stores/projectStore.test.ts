import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createRoot } from 'solid-js';
import { createProjectStore } from './projectStore';
import * as api from '../services/api';
import type { Project, CreateProjectDTO } from '../types/project';

vi.mock('../services/api');

describe('ProjectStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const mockProject: Project = {
    id: 1,
    name: 'Test Project',
    description: 'Test Description',
    local_path: '/home/user/test',
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  };

  it('should initialize with empty projects and not loading', () => {
    createRoot((dispose) => {
      const store = createProjectStore();

      expect(store.projects()).toEqual([]);
      expect(store.isLoading()).toBe(false);
      expect(store.error()).toBeNull();

      dispose();
    });
  });

  it('should load all projects', async () => {
    const mockProjects = [mockProject];
    vi.mocked(api.getAllProjects).mockResolvedValue(mockProjects);

    await createRoot(async (dispose) => {
      const store = createProjectStore();

      await store.loadProjects();

      expect(store.projects()).toEqual(mockProjects);
      expect(store.isLoading()).toBe(false);
      expect(store.error()).toBeNull();
      expect(api.getAllProjects).toHaveBeenCalled();

      dispose();
    });
  });

  it('should handle loading error', async () => {
    const errorMessage = 'Failed to load';
    vi.mocked(api.getAllProjects).mockRejectedValue(new Error(errorMessage));

    await createRoot(async (dispose) => {
      const store = createProjectStore();

      await store.loadProjects();

      expect(store.projects()).toEqual([]);
      expect(store.isLoading()).toBe(false);
      expect(store.error()).toBe(errorMessage);

      dispose();
    });
  });

  it('should create a new project', async () => {
    const newProjectDTO: CreateProjectDTO = {
      name: 'New Project',
      description: 'New Description',
      local_path: '/home/user/new',
    };

    vi.mocked(api.createProject).mockResolvedValue({
      id: 2,
      ...newProjectDTO,
      created_at: '2024-01-02T00:00:00Z',
      updated_at: '2024-01-02T00:00:00Z',
    });

    vi.mocked(api.getAllProjects).mockResolvedValue([mockProject]);

    await createRoot(async (dispose) => {
      const store = createProjectStore();
      await store.loadProjects();

      await store.createProject(newProjectDTO);

      expect(api.createProject).toHaveBeenCalledWith(newProjectDTO);
      expect(api.getAllProjects).toHaveBeenCalledTimes(2);

      dispose();
    });
  });

  it('should update a project', async () => {
    const updates = { name: 'Updated Name' };
    const updatedProject = { ...mockProject, ...updates };

    vi.mocked(api.updateProject).mockResolvedValue(updatedProject);
    vi.mocked(api.getAllProjects).mockResolvedValue([mockProject]);

    await createRoot(async (dispose) => {
      const store = createProjectStore();
      await store.loadProjects();

      await store.updateProject(1, updates);

      expect(api.updateProject).toHaveBeenCalledWith(1, updates);
      expect(api.getAllProjects).toHaveBeenCalledTimes(2);

      dispose();
    });
  });

  it('should delete a project', async () => {
    vi.mocked(api.deleteProject).mockResolvedValue();
    vi.mocked(api.getAllProjects).mockResolvedValue([mockProject]);

    await createRoot(async (dispose) => {
      const store = createProjectStore();
      await store.loadProjects();

      await store.deleteProject(1);

      expect(api.deleteProject).toHaveBeenCalledWith(1);
      expect(api.getAllProjects).toHaveBeenCalledTimes(2);

      dispose();
    });
  });

  it('should search projects', async () => {
    const searchResults = [mockProject];
    vi.mocked(api.searchProjects).mockResolvedValue(searchResults);

    await createRoot(async (dispose) => {
      const store = createProjectStore();

      await store.searchProjects('test');

      expect(store.projects()).toEqual(searchResults);
      expect(api.searchProjects).toHaveBeenCalledWith('test');

      dispose();
    });
  });

  it('should open terminal for a project', async () => {
    vi.mocked(api.openTerminal).mockResolvedValue();

    await createRoot(async (dispose) => {
      const store = createProjectStore();

      await store.openTerminal('/home/user/test');

      expect(api.openTerminal).toHaveBeenCalledWith('/home/user/test');

      dispose();
    });
  });
});
