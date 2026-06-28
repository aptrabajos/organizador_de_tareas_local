import { createSignal } from 'solid-js';
import { getErrorMessage } from '../utils/errors';
import type {
  Project,
  CreateProjectDTO,
  UpdateProjectDTO,
} from '../types/project';
import * as api from '../services/api';

export function createProjectStore() {
  const [projects, setProjects] = createSignal<Project[]>([]);
  const [isLoading, setIsLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  // Estado para navegación de grupos (v0.4.0)
  const [currentGroup, setCurrentGroup] = createSignal<Project | null>(null);
  const [viewMode, setViewMode] = createSignal<'groups' | 'subprojects'>(
    'groups'
  );

  async function loadProjects() {
    setIsLoading(true);
    setError(null);
    try {
      const data = await api.getAllProjects();
      setProjects(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Error desconocido');
    } finally {
      setIsLoading(false);
    }
  }

  async function createProject(project: CreateProjectDTO) {
    setIsLoading(true);
    setError(null);
    try {
      await api.createProject(project);
      // Recargar la vista actual para reflejar el nuevo proyecto
      if (viewMode() === 'groups') {
        await loadRootProjects();
      } else if (currentGroup()) {
        await loadSubprojects(currentGroup()!.id);
      } else {
        await loadRootProjects();
      }
    } catch (err) {
      setError(getErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }

  async function updateProject(id: number, updates: UpdateProjectDTO) {
    console.log(
      '🔧 [STORE] Iniciando actualización del proyecto:',
      id,
      updates
    );
    setIsLoading(true);
    setError(null);
    try {
      console.log('📡 [STORE] Llamando a API updateProject...');
      await api.updateProject(id, updates);
      console.log(
        '✅ [STORE] API updateProject exitosa, recargando proyectos...'
      );
      // Recargar la vista actual en lugar de todos los proyectos
      if (viewMode() === 'groups') {
        await loadRootProjects();
      } else if (currentGroup()) {
        await loadSubprojects(currentGroup()!.id);
      } else {
        // Fallback por si acaso
        await loadRootProjects();
      }
      console.log('✅ [STORE] Proyectos recargados exitosamente');
    } catch (err) {
      console.error('❌ [STORE] Error en updateProject:', err);
      setError(err instanceof Error ? err.message : 'Error desconocido');
      throw err;
    } finally {
      setIsLoading(false);
    }
  }

  async function deleteProject(id: number) {
    setIsLoading(true);
    setError(null);
    try {
      await api.deleteProject(id);
      // Recargar la vista actual para reflejar la eliminación
      if (viewMode() === 'groups') {
        await loadRootProjects();
      } else if (currentGroup()) {
        await loadSubprojects(currentGroup()!.id);
      } else {
        await loadRootProjects();
      }
    } catch (err) {
      setError(getErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }

  async function searchProjects(query: string) {
    setIsLoading(true);
    setError(null);
    try {
      const data = await api.searchProjects(query);
      setProjects(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Error desconocido');
    } finally {
      setIsLoading(false);
    }
  }

  async function openTerminal(path: string) {
    try {
      await api.openTerminal(path);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Error al abrir terminal');
      throw err;
    }
  }

  // ==================== FUNCIONES DE GRUPOS (v0.4.0) ====================

  async function loadRootProjects() {
    setIsLoading(true);
    setError(null);
    try {
      const data = await api.getRootProjects();
      setProjects(data);
      setViewMode('groups');
      setCurrentGroup(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Error desconocido');
    } finally {
      setIsLoading(false);
    }
  }

  async function loadSubprojects(parentId: number) {
    setIsLoading(true);
    setError(null);
    try {
      const data = await api.getSubprojects(parentId);
      setProjects(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Error desconocido');
    } finally {
      setIsLoading(false);
    }
  }

  async function navigateToGroup(group: Project) {
    setCurrentGroup(group);
    setViewMode('subprojects');
    await loadSubprojects(group.id);
  }

  async function navigateBack() {
    setCurrentGroup(null);
    setViewMode('groups');
    await loadRootProjects();
  }

  async function assignToGroup(childId: number, parentId: number | null) {
    setIsLoading(true);
    setError(null);
    try {
      await api.assignProjectToGroup(childId, parentId);
      // Recargar la vista actual
      if (viewMode() === 'groups') {
        await loadRootProjects();
      } else if (currentGroup()) {
        await loadSubprojects(currentGroup()!.id);
      }
    } catch (err) {
      setError(getErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }

  return {
    projects,
    isLoading,
    error,
    loadProjects,
    createProject,
    updateProject,
    deleteProject,
    searchProjects,
    openTerminal,
    // Grupos (v0.4.0)
    currentGroup,
    viewMode,
    loadRootProjects,
    loadSubprojects,
    navigateToGroup,
    navigateBack,
    assignToGroup,
  };
}

export type ProjectStore = ReturnType<typeof createProjectStore>;
