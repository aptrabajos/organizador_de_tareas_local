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

  // OJO con los rechazos de estos mocks: en Tauri v2 un comando que devuelve
  // `Err(String)` rechaza la promesa con un STRING PLANO, no con un `Error`.
  // Mockear con `new Error(...)` hacía pasar al patrón roto
  // `err instanceof Error ? err.message : 'Error desconocido'` y por eso el bug
  // sobrevivió. Los rechazos de acá imitan lo que hace Tauri de verdad.
  it('should handle loading error', async () => {
    const errorMessage = 'Failed to load';
    vi.mocked(api.getAllProjects).mockRejectedValue(errorMessage);

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

    vi.mocked(api.getRootProjects).mockResolvedValue([mockProject]);

    await createRoot(async (dispose) => {
      const store = createProjectStore();

      await store.createProject(newProjectDTO);

      expect(api.createProject).toHaveBeenCalledWith(newProjectDTO);
      // After create, store reloads via getRootProjects (groups mode)
      expect(api.getRootProjects).toHaveBeenCalled();

      dispose();
    });
  });

  it('should update a project', async () => {
    const updates = { name: 'Updated Name' };
    const updatedProject = { ...mockProject, ...updates };

    vi.mocked(api.updateProject).mockResolvedValue(updatedProject);
    vi.mocked(api.getRootProjects).mockResolvedValue([updatedProject]);

    await createRoot(async (dispose) => {
      const store = createProjectStore();

      await store.updateProject(1, updates);

      expect(api.updateProject).toHaveBeenCalledWith(1, updates);
      // After update, store reloads via getRootProjects (groups mode)
      expect(api.getRootProjects).toHaveBeenCalled();

      dispose();
    });
  });

  it('should delete a project', async () => {
    vi.mocked(api.deleteProject).mockResolvedValue();
    vi.mocked(api.getRootProjects).mockResolvedValue([]);

    await createRoot(async (dispose) => {
      const store = createProjectStore();

      await store.deleteProject(1);

      expect(api.deleteProject).toHaveBeenCalledWith(1);
      // After delete, store reloads via getRootProjects (groups mode)
      expect(api.getRootProjects).toHaveBeenCalled();

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

  // ==================== GRUPOS (v0.4.0) ====================

  describe('Groups', () => {
    it('should load root projects and set viewMode to groups', async () => {
      const mockRootProjects = [mockProject];
      vi.mocked(api.getRootProjects).mockResolvedValue(mockRootProjects);

      await createRoot(async (dispose) => {
        const store = createProjectStore();

        await store.loadRootProjects();

        expect(api.getRootProjects).toHaveBeenCalled();
        expect(store.projects()).toEqual(mockRootProjects);
        expect(store.viewMode()).toBe('groups');
        expect(store.currentGroup()).toBeNull();

        dispose();
      });
    });

    it('should navigate to group and load subprojects', async () => {
      const groupProject: Project = { ...mockProject, id: 10, name: 'Group A' };
      const mockSubprojects = [
        { ...mockProject, id: 20, name: 'Sub 1', parent_id: 10 },
        { ...mockProject, id: 21, name: 'Sub 2', parent_id: 10 },
      ];
      vi.mocked(api.getSubprojects).mockResolvedValue(mockSubprojects);

      await createRoot(async (dispose) => {
        const store = createProjectStore();

        await store.navigateToGroup(groupProject);

        expect(store.currentGroup()).toEqual(groupProject);
        expect(store.viewMode()).toBe('subprojects');
        expect(api.getSubprojects).toHaveBeenCalledWith(10);
        expect(store.projects()).toEqual(mockSubprojects);

        dispose();
      });
    });

    it('should navigate back to root groups', async () => {
      const mockRootProjects = [mockProject];
      vi.mocked(api.getRootProjects).mockResolvedValue(mockRootProjects);

      await createRoot(async (dispose) => {
        const store = createProjectStore();

        await store.navigateBack();

        expect(store.currentGroup()).toBeNull();
        expect(store.viewMode()).toBe('groups');
        expect(api.getRootProjects).toHaveBeenCalled();
        expect(store.projects()).toEqual(mockRootProjects);

        dispose();
      });
    });

    it('should load subprojects for a parent', async () => {
      const mockSubprojects = [
        { ...mockProject, id: 20, name: 'Sub 1', parent_id: 5 },
      ];
      vi.mocked(api.getSubprojects).mockResolvedValue(mockSubprojects);

      await createRoot(async (dispose) => {
        const store = createProjectStore();

        await store.loadSubprojects(5);

        expect(api.getSubprojects).toHaveBeenCalledWith(5);
        expect(store.projects()).toEqual(mockSubprojects);

        dispose();
      });
    });

    it('should set error when loadSubprojects fails', async () => {
      vi.mocked(api.getSubprojects).mockRejectedValue('DB error');

      await createRoot(async (dispose) => {
        const store = createProjectStore();

        await store.loadSubprojects(5);

        expect(store.error()).toBe('DB error');
        expect(store.projects()).toEqual([]);

        dispose();
      });
    });

    it('should reload subprojects after creating project in subprojects mode', async () => {
      const groupProject: Project = { ...mockProject, id: 10, name: 'Group A' };
      const mockSubprojects = [
        { ...mockProject, id: 20, name: 'Sub 1', parent_id: 10 },
      ];

      vi.mocked(api.getSubprojects).mockResolvedValue(mockSubprojects);
      vi.mocked(api.createProject).mockResolvedValue({
        ...mockProject,
        id: 30,
      });

      await createRoot(async (dispose) => {
        const store = createProjectStore();

        // Navigate to group first
        await store.navigateToGroup(groupProject);
        vi.clearAllMocks();

        // Mock for the reload after create
        vi.mocked(api.getSubprojects).mockResolvedValue(mockSubprojects);
        vi.mocked(api.createProject).mockResolvedValue({
          ...mockProject,
          id: 30,
        });

        await store.createProject({
          name: 'New Sub',
          description: 'Desc',
          local_path: '/new',
        });

        expect(api.createProject).toHaveBeenCalled();
        expect(api.getSubprojects).toHaveBeenCalledWith(10);

        dispose();
      });
    });

    it('should reload root projects after deleting project in groups mode', async () => {
      const mockRootProjects = [mockProject];
      vi.mocked(api.getRootProjects).mockResolvedValue(mockRootProjects);
      vi.mocked(api.deleteProject).mockResolvedValue();

      await createRoot(async (dispose) => {
        const store = createProjectStore();

        // Load root projects first (sets viewMode to 'groups')
        await store.loadRootProjects();
        vi.clearAllMocks();

        vi.mocked(api.getRootProjects).mockResolvedValue([]);
        vi.mocked(api.deleteProject).mockResolvedValue();

        await store.deleteProject(1);

        expect(api.deleteProject).toHaveBeenCalledWith(1);
        expect(api.getRootProjects).toHaveBeenCalled();

        dispose();
      });
    });
  });

  // ==================== MENSAJES DEL BACKEND (Tauri v2) ====================

  describe('Propagación de mensajes del backend', () => {
    // En Tauri v2, `Err(String)` del lado Rust llega al front como un string plano.
    // El backend escribe estas validaciones en español PARA EL USUARIO, así que el
    // store tiene que dejarlas pasar tal cual: si aparece "Error desconocido",
    // alguien volvió a meter `err instanceof Error ? err.message : '...'`.
    const backendMessage =
      'No podés asignar un proyecto como su propio grupo padre.';

    it('conserva el texto original en loadProjects', async () => {
      vi.mocked(api.getAllProjects).mockRejectedValue(backendMessage);

      await createRoot(async (dispose) => {
        const store = createProjectStore();
        await store.loadProjects();

        expect(store.error()).toBe(backendMessage);
        expect(store.error()).not.toBe('Error desconocido');

        dispose();
      });
    });

    it('conserva el texto original en updateProject', async () => {
      vi.mocked(api.updateProject).mockRejectedValue(backendMessage);

      await createRoot(async (dispose) => {
        const store = createProjectStore();
        await expect(store.updateProject(1, { name: 'X' })).rejects.toBe(
          backendMessage
        );

        expect(store.error()).toBe(backendMessage);

        dispose();
      });
    });

    it('conserva el texto original en searchProjects', async () => {
      vi.mocked(api.searchProjects).mockRejectedValue(backendMessage);

      await createRoot(async (dispose) => {
        const store = createProjectStore();
        await store.searchProjects('algo');

        expect(store.error()).toBe(backendMessage);

        dispose();
      });
    });

    it('conserva el texto original en loadRootProjects', async () => {
      vi.mocked(api.getRootProjects).mockRejectedValue(backendMessage);

      await createRoot(async (dispose) => {
        const store = createProjectStore();
        await store.loadRootProjects();

        expect(store.error()).toBe(backendMessage);

        dispose();
      });
    });

    it('conserva el texto original en assignToGroup', async () => {
      vi.mocked(api.assignProjectToGroup).mockRejectedValue(backendMessage);

      await createRoot(async (dispose) => {
        const store = createProjectStore();
        await expect(store.assignToGroup(1, 1)).rejects.toBe(backendMessage);

        expect(store.error()).toBe(backendMessage);

        dispose();
      });
    });

    it('conserva el texto original en openTerminal', async () => {
      const terminalMessage = 'Ruta de terminal personalizado no configurada';
      vi.mocked(api.openTerminal).mockRejectedValue(terminalMessage);

      await createRoot(async (dispose) => {
        const store = createProjectStore();
        await expect(store.openTerminal('/tmp')).rejects.toBe(terminalMessage);

        expect(store.error()).toBe(terminalMessage);
        expect(store.error()).not.toBe('Error al abrir terminal');

        dispose();
      });
    });

    it('cae en "Error desconocido" solo si el rechazo no es string ni Error', async () => {
      vi.mocked(api.getAllProjects).mockRejectedValue({ code: 500 });

      await createRoot(async (dispose) => {
        const store = createProjectStore();
        await store.loadProjects();

        expect(store.error()).toBe('Error desconocido');

        dispose();
      });
    });
  });
});
