import { Component, createSignal, onMount, Show } from 'solid-js';
import { getErrorMessage } from './utils/errors';
import { Toaster } from 'solid-toast';
import { createProjectStore } from './stores/projectStore';
import SearchBar from './components/SearchBar';
import ProjectList from './components/ProjectList';
import ProjectFormTabs from './components/ProjectFormTabs';
import { type ProjectFormData } from './components/ProjectForm';
import ThemeToggle from './components/ThemeToggle';
import Settings from './components/Settings';
import WelcomeScreen from './components/WelcomeScreen';
import About from './components/About';
import TrashModal from './components/TrashModal';
import ProjectFilters from './components/ProjectFilters';
import type { ProjectFiltersProps } from './components/ProjectFilters';
import TreeView from './components/TreeView';
import Dashboard from './components/Dashboard';
import { ThemeProvider } from './contexts/ThemeContext';
import { ConfigProvider, useConfig } from './contexts/ConfigContext';
import { ShortcutsProvider, useShortcuts } from './contexts/ShortcutsContext';
import { shouldConfirm } from './utils/confirm';
import type { Project } from './types/project';
import { confirm } from '@tauri-apps/plugin-dialog';
import { getConfig, countSubprojects } from './services/api';
import { logger } from './utils/logger';

// Componente interno que usa shortcuts
const AppContent: Component = () => {
  const store = createProjectStore();
  const shortcuts = useShortcuts();
  const configCtx = useConfig();
  // Entrar a un grupo: salir del Dashboard (el store sale de la búsqueda por sí solo).
  const enterGroup = (group: Project) => {
    setShowDashboard(false);
    store.navigateToGroup(group);
  };
  const [showForm, setShowForm] = createSignal(false);
  const [showSettings, setShowSettings] = createSignal(false);
  const [showWelcome, setShowWelcome] = createSignal(false);
  const [showTreeView, setShowTreeView] = createSignal(false);
  const [showAbout, setShowAbout] = createSignal(false);
  const [showTrash, setShowTrash] = createSignal(false);
  const [showDashboard, setShowDashboard] = createSignal(true);
  const [editingProject, setEditingProject] = createSignal<Project | null>(
    null
  );

  // Estado para los filtros (recibidos desde ProjectList)
  const [filterProps, setFilterProps] =
    createSignal<ProjectFiltersProps | null>(null);

  onMount(async () => {
    // Cargar proyectos raíz (grupos) por defecto (v0.4.0)
    store.loadRootProjects();

    // Verificar si mostrar welcome screen
    try {
      const config = await getConfig();
      if (config.ui.show_welcome) {
        setShowWelcome(true);
      }
    } catch (err) {
      logger.error('Error cargando config:', err);
    }

    // Registrar handlers de shortcuts
    shortcuts.registerHandler('new_project', () => {
      logger.debug('🎯 [SHORTCUT] Nuevo proyecto');
      setShowForm(true);
      setEditingProject(null);
    });

    shortcuts.registerHandler('search', () => {
      logger.debug('🎯 [SHORTCUT] Focus en búsqueda');
      const searchInput = document.querySelector(
        'input[type="text"]'
      ) as HTMLInputElement;
      if (searchInput) {
        searchInput.focus();
      }
    });

    shortcuts.registerHandler('settings', () => {
      logger.debug('🎯 [SHORTCUT] Abrir configuración');
      setShowSettings(true);
    });

    shortcuts.registerHandler('about', () => {
      logger.debug('🎯 [SHORTCUT] Abrir acerca de');
      setShowAbout(true);
    });

    shortcuts.registerHandler('refresh', () => {
      logger.debug('🎯 [SHORTCUT] Recargar proyectos');
      if (store.viewMode() === 'groups') {
        store.loadRootProjects();
      } else if (store.currentGroup()) {
        store.loadSubprojects(store.currentGroup()!.id);
      }
    });

    shortcuts.registerHandler('close_modal', () => {
      logger.debug('🎯 [SHORTCUT] Cerrar modal');
      if (showForm()) {
        setShowForm(false);
        setEditingProject(null);
      } else if (showSettings()) {
        setShowSettings(false);
      } else if (showAbout()) {
        setShowAbout(false);
      } else if (showTrash()) {
        setShowTrash(false);
      }
    });

    logger.debug(
      '🚀 [SHORTCUTS] Todos los handlers registrados, activando shortcuts globales'
    );
    shortcuts.reregisterShortcuts().catch((err) => {
      logger.error('❌ [SHORTCUTS] Error registrando shortcuts:', err);
    });
  });

  const handleSearch = (query: string) => {
    // El store es el dueño de la query y recarga respetándola (fuente única)
    store.search(query);
  };

  const handleNewProject = () => {
    setEditingProject(null);
    setShowForm(true);
  };

  const handleEdit = (project: Project) => {
    setEditingProject(project);
    setShowForm(true);
  };

  const handleDelete = async (project: Project) => {
    // Ahora el borrado es reversible (papelera): el copy lo refleja, y si es un grupo
    // con subproyectos avisa que se arrastran a la papelera.
    let message = `¿Mover "${project.name}" a la papelera? Vas a poder restaurarlo después.`;
    try {
      const n = await countSubprojects(project.id);
      if (n > 0) {
        message = `Esto mueve "${project.name}" y sus ${n} subproyecto(s) a la papelera. Vas a poder restaurarlos después.`;
      }
    } catch {
      // si falla el conteo, seguimos con el mensaje base
    }

    // Mover a la papelera es REVERSIBLE (soft-delete), así que respeta
    // `ui.confirm_delete`. El purgado definitivo vive en TrashModal y confirma
    // siempre, sin importar el flag.
    if (shouldConfirm('reversible', configCtx.config())) {
      const confirmed = await confirm(message, {
        title: 'Mover a la papelera',
        kind: 'warning',
      });
      if (!confirmed) return;
    }

    try {
      // Si borramos el grupo en el que estamos parados, salimos de su vista
      // para no quedar mostrando una cabecera de un grupo ya eliminado.
      const wasCurrentGroup = project.id === store.currentGroup()?.id;
      await store.deleteProject(project.id);
      if (wasCurrentGroup) {
        await store.navigateBack();
      }
    } catch {
      alert('Error al mover el proyecto a la papelera');
    }
  };

  const handleOpenTerminal = async (project: Project) => {
    try {
      await store.openTerminal(project.local_path);
    } catch {
      alert('Error al abrir terminal');
    }
  };

  const handleFormSubmit = async (data: ProjectFormData) => {
    logger.debug('🔧 [APP] handleFormSubmit iniciado con datos:', data);
    logger.debug('🔧 [APP] editingProject:', editingProject());
    try {
      if (editingProject()) {
        logger.debug('🔧 [APP] Llamando a store.updateProject...');
        const editing = editingProject()!;
        const id = editing.id;
        // parent_id necesita NULL real para poder DESAGRUPAR: se rutea por
        // assignToGroup (Option<i64>-aware), no por update_project (que saltea None).
        const { parent_id, ...rest } = data;
        await store.updateProject(id, rest);
        // Solo re-rutear el grupo si realmente cambió: evita un UPDATE no-op y una
        // segunda recarga de vista (parpadeo) en cada edición.
        const newParent = parent_id ?? null;
        const oldParent = editing.parent_id ?? null;
        if (newParent !== oldParent) {
          await store.assignToGroup(id, newParent);
        }
        logger.debug('✅ [APP] store.updateProject exitoso');
      } else {
        logger.debug('🔧 [APP] Llamando a store.createProject...');
        await store.createProject(data);
        logger.debug('✅ [APP] store.createProject exitoso');
      }
      logger.debug('🔧 [APP] Cerrando formulario...');
      setShowForm(false);
      setEditingProject(null);
      logger.debug('✅ [APP] Formulario cerrado exitosamente');
    } catch (err) {
      logger.error('❌ [APP] Error en handleFormSubmit:', err);
      alert('Error al guardar el proyecto: ' + getErrorMessage(err));
    }
  };

  const handleFormCancel = () => {
    setShowForm(false);
    setEditingProject(null);
  };

  return (
    <div class="min-h-screen bg-surface-50 transition-colors duration-300 dark:bg-surface-950">
      <Toaster
        position="top-right"
        toastOptions={{
          className:
            '!bg-white dark:!bg-surface-800 !text-surface-900 dark:!text-surface-100 !shadow-lg !border !border-surface-200 dark:!border-surface-700 !rounded-xl',
        }}
      />
      <ThemeToggle />

      {/* ═══════════════════════════════════════════════════════════════════════
          HEADER - Developer Command Center Style
          ═══════════════════════════════════════════════════════════════════════ */}
      <header class="app-header">
        <div class="px-4 py-3 sm:px-6">
          {/* Primera fila: Logo/Título + Navegación principal */}
          <div class="flex items-center justify-between gap-4">
            {/* Logo y título */}
            <div class="flex items-center gap-3">
              <div class="flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-accent-500 to-emerald-500 text-lg shadow-glow-sm">
                <span class="font-bold text-white">GP</span>
              </div>
              <div>
                <h1 class="text-lg font-bold tracking-tight text-surface-900 dark:text-white">
                  Gestor de Proyectos
                </h1>
                <p class="text-xs text-surface-500 dark:text-surface-400">
                  Developer Command Center
                </p>
              </div>
            </div>

            {/* Navegación principal */}
            <nav class="flex items-center gap-2">
              <button
                onClick={() => {
                  if (store.isSearchActive()) {
                    // Con búsqueda activa el botón sale de la búsqueda y muestra el
                    // Dashboard (efecto visible inmediato, sin estado fantasma).
                    store.search('');
                    setShowDashboard(true);
                  } else {
                    setShowDashboard(!showDashboard());
                  }
                }}
                class={`btn-ghost group relative flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-all ${
                  showDashboard() && !store.isSearchActive()
                    ? 'bg-accent-500/10 text-accent-600 dark:bg-accent-400/10 dark:text-accent-400'
                    : ''
                }`}
              >
                <svg
                  class="h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"
                  />
                </svg>
                <span class="hidden sm:inline">Dashboard</span>
              </button>

              <button
                onClick={() => setShowTreeView(!showTreeView())}
                class={`btn-ghost group relative flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-all ${
                  showTreeView()
                    ? 'bg-emerald-500/10 text-emerald-600 dark:bg-emerald-400/10 dark:text-emerald-400'
                    : ''
                }`}
              >
                <svg
                  class="h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M4 6h16M4 10h16M4 14h16M4 18h16"
                  />
                </svg>
                <span class="hidden sm:inline">Árbol</span>
              </button>

              <button
                onClick={() => setShowTrash(true)}
                class="btn-ghost group relative flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-all"
                title="Papelera"
              >
                <svg
                  class="h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                  />
                </svg>
                <span class="hidden sm:inline">Papelera</span>
              </button>

              <div class="mx-2 h-6 w-px bg-surface-200 dark:bg-surface-700" />

              <button
                onClick={() => setShowSettings(true)}
                class="btn-icon"
                title="Configuración (Ctrl+,)"
              >
                <svg
                  class="h-5 w-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                  />
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                  />
                </svg>
              </button>

              <button
                onClick={() => setShowAbout(true)}
                class="btn-icon"
                title="Acerca de (Ctrl+Shift+A)"
              >
                <svg
                  class="h-5 w-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
              </button>

              <button onClick={handleNewProject} class="btn-primary ml-2">
                <svg
                  class="h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 4v16m8-8H4"
                  />
                </svg>
                <span class="hidden sm:inline">Nuevo</span>
              </button>
            </nav>
          </div>

          {/* Segunda fila: Breadcrumb + Búsqueda y Filtros */}
          <Show
            when={
              store.viewMode() === 'subprojects' &&
              store.currentGroup() &&
              !showDashboard()
            }
          >
            <div class="mt-3 flex items-center gap-2">
              <button
                onClick={() => store.navigateBack()}
                class="inline-flex items-center gap-1.5 rounded-lg bg-surface-100 px-3 py-1.5 text-sm font-medium text-surface-700 transition-all hover:bg-surface-200 dark:bg-surface-800 dark:text-surface-300 dark:hover:bg-surface-700"
              >
                <svg
                  class="h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M10 19l-7-7m0 0l7-7m-7 7h18"
                  />
                </svg>
                Grupos
              </button>
              <span class="text-surface-400 dark:text-surface-600">/</span>
              <span class="badge-accent">
                <span class="mr-1">📁</span>
                {store.currentGroup()?.name}
              </span>
            </div>
          </Show>

          <div class="mt-3 flex flex-wrap items-center gap-3">
            <div class="min-w-[280px] flex-1">
              <SearchBar onSearch={handleSearch} value={store.searchQuery()} />
            </div>
            <Show when={filterProps()}>
              <ProjectFilters {...filterProps()!} />
            </Show>
          </div>
        </div>
      </header>

      {/* ═══════════════════════════════════════════════════════════════════════
          MAIN CONTENT
          ═══════════════════════════════════════════════════════════════════════ */}
      <main class="min-h-[calc(100vh-140px)]">
        <Show
          when={showDashboard() && !store.isSearchActive()}
          fallback={
            <div class="p-4 sm:p-6">
              <div class="flex gap-6">
                {/* TreeView Sidebar */}
                <Show when={showTreeView()}>
                  <aside class="hidden w-72 flex-shrink-0 lg:block xl:w-80">
                    <div class="sticky top-24">
                      <TreeView
                        // Mismo trigger que reutiliza reloadCurrentView: cualquier
                        // mutación (pin, estado, drag-drop, commit, etc.) incrementa
                        // dataVersion en el store y esto obliga a recargar el árbol
                        // en vez de quedar stale desde el onMount inicial.
                        refreshToken={store.dataVersion()}
                        onSelectProject={(project) => {
                          if (!project.parent_id) {
                            enterGroup(project);
                          } else {
                            handleEdit(project);
                          }
                        }}
                      />
                    </div>
                  </aside>
                </Show>

                {/* Main Content Area */}
                <div class="flex-1">
                  {/* Error Message */}
                  <Show when={store.error()}>
                    <div class="mb-6 flex items-center gap-3 rounded-xl border border-rose-200 bg-rose-50 p-4 dark:border-rose-800 dark:bg-rose-900/20">
                      <div class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full bg-rose-100 dark:bg-rose-900/50">
                        <svg
                          class="h-5 w-5 text-rose-600 dark:text-rose-400"
                          fill="none"
                          viewBox="0 0 24 24"
                          stroke="currentColor"
                        >
                          <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                          />
                        </svg>
                      </div>
                      <div>
                        <p class="font-medium text-rose-800 dark:text-rose-200">
                          Error
                        </p>
                        <p class="text-sm text-rose-600 dark:text-rose-300">
                          {store.error()}
                        </p>
                      </div>
                    </div>
                  </Show>

                  {/* Loading State */}
                  <Show when={store.isLoading()}>
                    <div class="flex flex-col items-center justify-center py-16">
                      <div class="spinner mb-4 h-8 w-8" />
                      <p class="text-sm text-surface-500 dark:text-surface-400">
                        Cargando proyectos...
                      </p>
                    </div>
                  </Show>

                  {/* Project List */}
                  <Show when={!store.isLoading()}>
                    <ProjectList
                      projects={store.projects()}
                      onEdit={handleEdit}
                      onDelete={handleDelete}
                      onOpenTerminal={handleOpenTerminal}
                      onProjectsChanged={() => {
                        // Fuente única: reloadCurrentView() ya respeta la búsqueda
                        // activa (isSearchActive) además de grupos/subproyectos, a
                        // diferencia de la reimplementación parcial que había acá.
                        store.reloadCurrentView();
                        if (
                          store.viewMode() === 'subprojects' &&
                          store.currentGroup()
                        ) {
                          // Re-hidratar la cabecera del grupo por si se mutó desde ahí
                          store.refreshCurrentGroup();
                        }
                      }}
                      renderFilters={setFilterProps}
                      viewMode={store.viewMode()}
                      onViewGroup={(group) => enterGroup(group)}
                      searchActive={store.isSearchActive()}
                      currentGroup={store.currentGroup()}
                    />
                  </Show>
                </div>
              </div>
            </div>
          }
        >
          <Dashboard
            onProjectClick={(project) => {
              // La búsqueda es un overlay: NO apagamos el Dashboard, así limpiar la
              // búsqueda vuelve al Dashboard igual que al tipear en el SearchBar (consistente).
              store.search(project.name);
            }}
          />
        </Show>
      </main>

      {/* ═══════════════════════════════════════════════════════════════════════
          MODALS
          ═══════════════════════════════════════════════════════════════════════ */}

      {/* Modal Form */}
      <Show when={showForm()}>
        <div class="modal-overlay">
          <div class="modal-content max-w-lg">
            <div class="mb-6 flex items-center justify-between">
              <h2 class="text-xl font-bold text-surface-900 dark:text-white">
                {editingProject() ? 'Editar Proyecto' : 'Nuevo Proyecto'}
              </h2>
              <button
                onClick={handleFormCancel}
                class="btn-icon-sm"
                aria-label="Cerrar"
              >
                <svg
                  class="h-5 w-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </button>
            </div>
            <ProjectFormTabs
              project={editingProject() || undefined}
              onSubmit={handleFormSubmit}
              onCancel={handleFormCancel}
            />
          </div>
        </div>
      </Show>

      {/* Modal Settings */}
      <Show when={showSettings()}>
        <Settings onClose={() => setShowSettings(false)} />
      </Show>

      {/* Welcome Screen */}
      <Show when={showWelcome()}>
        <WelcomeScreen onClose={() => setShowWelcome(false)} />
      </Show>

      {/* About Modal */}
      <Show when={showAbout()}>
        <About onClose={() => setShowAbout(false)} />
      </Show>

      {/* Trash Modal */}
      <Show when={showTrash()}>
        <TrashModal store={store} onClose={() => setShowTrash(false)} />
      </Show>
    </div>
  );
};

// Componente principal envuelto con providers.
// ConfigProvider va ARRIBA de ThemeProvider: el tema ahora se persiste en la
// config de Rust, así que el contexto de tema depende del de config.
const App: Component = () => {
  return (
    <ConfigProvider>
      <ThemeProvider>
        <ShortcutsProvider>
          <AppContent />
        </ShortcutsProvider>
      </ThemeProvider>
    </ConfigProvider>
  );
};

export default App;
