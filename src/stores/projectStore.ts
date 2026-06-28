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

  // Búsqueda centralizada en el store: TODA recarga la respeta vía reloadCurrentView,
  // así el SearchBar y la lista nunca se desincronizan (navegar, pin, CRUD, etc.).
  const [searchQuery, setSearchQuery] = createSignal('');
  const isSearchActive = () => searchQuery().trim().length > 0;

  // Fuente ÚNICA de verdad de "qué proyectos mostrar": si hay búsqueda activa muestra
  // resultados; si no, la vista actual (grupos raíz o subproyectos del grupo).
  async function reloadCurrentView() {
    if (isSearchActive()) {
      await searchProjects(searchQuery());
    } else if (viewMode() === 'groups') {
      await loadRootProjects();
    } else if (currentGroup()) {
      await loadSubprojects(currentGroup()!.id);
    } else {
      await loadRootProjects();
    }
  }

  // Setear la query y recargar en un solo paso (lo usa el SearchBar).
  async function search(query: string) {
    setSearchQuery(query);
    await reloadCurrentView();
  }

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
      // Recargar respetando la búsqueda/vista activa (fuente única)
      await reloadCurrentView();
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
      // Recargar respetando la búsqueda/vista activa (fuente única)
      await reloadCurrentView();
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
      // Recargar respetando la búsqueda/vista activa (fuente única)
      await reloadCurrentView();
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
    setSearchQuery(''); // navegar a un grupo siempre sale de la búsqueda
    setCurrentGroup(group);
    setViewMode('subprojects');
    await loadSubprojects(group.id);
  }

  async function navigateBack() {
    setSearchQuery(''); // volver a la raíz también sale de la búsqueda
    setCurrentGroup(null);
    setViewMode('groups');
    await loadRootProjects();
  }

  async function assignToGroup(childId: number, parentId: number | null) {
    setIsLoading(true);
    setError(null);
    try {
      await api.assignProjectToGroup(childId, parentId);
      // Recargar respetando la búsqueda/vista activa (fuente única)
      await reloadCurrentView();
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
    // Búsqueda centralizada (v0.4.5)
    searchQuery,
    setSearchQuery,
    isSearchActive,
    search,
    reloadCurrentView,
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
