import { createSignal } from 'solid-js';
import { getErrorMessage } from '../utils/errors';
import type {
  Project,
  CreateProjectDTO,
  UpdateProjectDTO,
} from '../types/project';
import * as api from '../services/api';
import { logger } from '../utils/logger';

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

  // Contador de versión de datos: se incrementa cada vez que projects() se repuebla
  // desde una carga real (root/subproyectos/búsqueda). Consumidores externos al store
  // (p.ej. TreeView) lo usan como trigger reactivo para resincronizarse sin necesitar
  // su propio mecanismo ad-hoc de "onProjectsChanged".
  const [dataVersion, setDataVersion] = createSignal(0);

  // Guard de orden de resolución: cada carga que puebla `projects` (root/subproyectos/
  // búsqueda) se identifica con un id incremental. Si al resolver ya no es la última
  // solicitud en vuelo, el resultado se descarta (evita que una respuesta vieja pise a
  // una más nueva, p.ej. al tipear rápido en el buscador o navegar durante una carga).
  let latestLoadRequestId = 0;

  // Debounce de búsqueda: evita disparar un invoke por cada tecla tipeada.
  const SEARCH_DEBOUNCE_MS = 200;
  let searchDebounceTimer: ReturnType<typeof setTimeout> | undefined;

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
  // La query se refleja de inmediato (input controlado sin lag), pero la recarga
  // real (invoke) se debouncea para no disparar un request por cada tecla.
  async function search(query: string) {
    setSearchQuery(query);
    if (searchDebounceTimer !== undefined) {
      clearTimeout(searchDebounceTimer);
    }
    await new Promise<void>((resolve) => {
      searchDebounceTimer = setTimeout(() => {
        searchDebounceTimer = undefined;
        reloadCurrentView().finally(resolve);
      }, SEARCH_DEBOUNCE_MS);
    });
  }

  async function loadProjects() {
    setIsLoading(true);
    setError(null);
    try {
      const data = await api.getAllProjects();
      setProjects(data);
    } catch (err) {
      setError(getErrorMessage(err));
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
    logger.debug(
      '🔧 [STORE] Iniciando actualización del proyecto:',
      id,
      updates
    );
    setIsLoading(true);
    setError(null);
    try {
      logger.debug('📡 [STORE] Llamando a API updateProject...');
      await api.updateProject(id, updates);
      logger.debug(
        '✅ [STORE] API updateProject exitosa, recargando proyectos...'
      );
      // Recargar respetando la búsqueda/vista activa (fuente única)
      await reloadCurrentView();
      logger.debug('✅ [STORE] Proyectos recargados exitosamente');
    } catch (err) {
      logger.error('❌ [STORE] Error en updateProject:', err);
      setError(getErrorMessage(err));
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

  async function restoreProject(id: number) {
    setIsLoading(true);
    setError(null);
    try {
      await api.restoreProject(id);
      // Recargar respetando la búsqueda/vista activa (fuente única)
      await reloadCurrentView();
    } catch (err) {
      setError(getErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }

  async function purgeProject(id: number) {
    setIsLoading(true);
    setError(null);
    try {
      await api.purgeProject(id);
      // Borrado definitivo: la vista activa no cambia, pero refrescamos por consistencia
      await reloadCurrentView();
    } catch (err) {
      setError(getErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }

  async function emptyTrash() {
    setIsLoading(true);
    setError(null);
    try {
      await api.emptyTrash();
      await reloadCurrentView();
    } catch (err) {
      setError(getErrorMessage(err));
      throw err;
    } finally {
      setIsLoading(false);
    }
  }

  async function searchProjects(query: string) {
    const requestId = ++latestLoadRequestId;
    setIsLoading(true);
    setError(null);
    try {
      const data = await api.searchProjects(query);
      if (requestId !== latestLoadRequestId) return; // respuesta obsoleta, descartar
      setProjects(data);
      setDataVersion((v) => v + 1);
    } catch (err) {
      if (requestId !== latestLoadRequestId) return;
      setError(getErrorMessage(err));
    } finally {
      if (requestId === latestLoadRequestId) {
        setIsLoading(false);
      }
    }
  }

  async function openTerminal(path: string) {
    try {
      await api.openTerminal(path);
    } catch (err) {
      setError(getErrorMessage(err));
      throw err;
    }
  }

  // ==================== FUNCIONES DE GRUPOS (v0.4.0) ====================

  async function loadRootProjects() {
    const requestId = ++latestLoadRequestId;
    setIsLoading(true);
    setError(null);
    try {
      const data = await api.getRootProjects();
      if (requestId !== latestLoadRequestId) return; // respuesta obsoleta, descartar
      setProjects(data);
      setViewMode('groups');
      setCurrentGroup(null);
      setDataVersion((v) => v + 1);
    } catch (err) {
      if (requestId !== latestLoadRequestId) return;
      setError(getErrorMessage(err));
    } finally {
      if (requestId === latestLoadRequestId) {
        setIsLoading(false);
      }
    }
  }

  async function loadSubprojects(parentId: number) {
    const requestId = ++latestLoadRequestId;
    setIsLoading(true);
    setError(null);
    try {
      const data = await api.getSubprojects(parentId);
      if (requestId !== latestLoadRequestId) return; // respuesta obsoleta, descartar
      setProjects(data);
      setDataVersion((v) => v + 1);
    } catch (err) {
      if (requestId !== latestLoadRequestId) return;
      setError(getErrorMessage(err));
    } finally {
      if (requestId === latestLoadRequestId) {
        setIsLoading(false);
      }
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

  // Re-hidrata currentGroup desde la DB. Se usa tras mutar el grupo desde su propia
  // cabecera (opción A) para que la tarjeta no quede con datos viejos (pin/estado/nombre).
  async function refreshCurrentGroup() {
    const g = currentGroup();
    if (!g) return;
    try {
      const fresh = await api.getProject(g.id);
      setCurrentGroup(fresh);
    } catch {
      // Si no se puede refrescar, se conserva el snapshot actual.
    }
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
    restoreProject,
    purgeProject,
    emptyTrash,
    searchProjects,
    // Búsqueda centralizada (v0.4.5)
    searchQuery,
    setSearchQuery,
    isSearchActive,
    search,
    reloadCurrentView,
    dataVersion,
    openTerminal,
    // Grupos (v0.4.0)
    currentGroup,
    viewMode,
    loadRootProjects,
    loadSubprojects,
    navigateToGroup,
    navigateBack,
    refreshCurrentGroup,
    assignToGroup,
  };
}

export type ProjectStore = ReturnType<typeof createProjectStore>;
