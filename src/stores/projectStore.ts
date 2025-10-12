import { createSignal } from 'solid-js';
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
      await loadProjects();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Error desconocido');
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
      await loadProjects();
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
      await loadProjects();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Error desconocido');
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
  };
}

export type ProjectStore = ReturnType<typeof createProjectStore>;
