import { Component, createSignal, onMount, Show } from 'solid-js';
import { Toaster } from 'solid-toast';
import { createProjectStore } from './stores/projectStore';
import SearchBar from './components/SearchBar';
import ProjectList from './components/ProjectList';
import ProjectFormTabs from './components/ProjectFormTabs';
import { type ProjectFormData } from './components/ProjectForm';
import ThemeToggle from './components/ThemeToggle';
import Analytics from './components/Analytics';
import Settings from './components/Settings';
import WelcomeScreen from './components/WelcomeScreen';
import ProjectFilters from './components/ProjectFilters';
import TreeView from './components/TreeView';
import { ThemeProvider } from './contexts/ThemeContext';
import { ShortcutsProvider, useShortcuts } from './contexts/ShortcutsContext';
import type { Project } from './types/project';
import { confirm } from '@tauri-apps/plugin-dialog';
import { getConfig } from './services/api';

// Componente interno que usa shortcuts
const AppContent: Component = () => {
  const store = createProjectStore();
  const shortcuts = useShortcuts();
  const [searchQuery, setSearchQuery] = createSignal('');
  const [showForm, setShowForm] = createSignal(false);
  const [showAnalytics, setShowAnalytics] = createSignal(false);
  const [showSettings, setShowSettings] = createSignal(false);
  const [showWelcome, setShowWelcome] = createSignal(false);
  const [showTreeView, setShowTreeView] = createSignal(false);
  const [editingProject, setEditingProject] = createSignal<Project | null>(
    null
  );

  // Estado para los filtros (recibidos desde ProjectList)
  const [filterProps, setFilterProps] = createSignal<any>(null);

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
      console.error('Error cargando config:', err);
    }

    // Registrar handlers de shortcuts
    shortcuts.registerHandler('new_project', () => {
      console.log('🎯 [SHORTCUT] Nuevo proyecto');
      setShowForm(true);
      setEditingProject(null);
    });

    shortcuts.registerHandler('search', () => {
      console.log('🎯 [SHORTCUT] Focus en búsqueda');
      const searchInput = document.querySelector(
        'input[type="text"]'
      ) as HTMLInputElement;
      if (searchInput) {
        searchInput.focus();
      }
    });

    shortcuts.registerHandler('settings', () => {
      console.log('🎯 [SHORTCUT] Abrir configuración');
      setShowSettings(true);
    });

    shortcuts.registerHandler('analytics', () => {
      console.log('🎯 [SHORTCUT] Toggle analytics');
      setShowAnalytics((prev) => !prev);
    });

    shortcuts.registerHandler('refresh', () => {
      console.log('🎯 [SHORTCUT] Recargar proyectos');
      // Recargar vista actual (v0.4.0)
      if (store.viewMode() === 'groups') {
        store.loadRootProjects();
      } else if (store.currentGroup()) {
        store.loadSubprojects(store.currentGroup()!.id);
      }
    });

    shortcuts.registerHandler('close_modal', () => {
      console.log('🎯 [SHORTCUT] Cerrar modal');
      // Cerrar cualquier modal activo
      if (showForm()) {
        setShowForm(false);
        setEditingProject(null);
      } else if (showSettings()) {
        setShowSettings(false);
      } else if (showAnalytics()) {
        setShowAnalytics(false);
      }
    });

    // Después de registrar todos los handlers, activar los shortcuts globales
    console.log(
      '🚀 [SHORTCUTS] Todos los handlers registrados, activando shortcuts globales'
    );
    shortcuts.reregisterShortcuts().catch((err) => {
      console.error('❌ [SHORTCUTS] Error registrando shortcuts:', err);
    });
  });

  const handleSearch = (query: string) => {
    setSearchQuery(query);
    if (query.trim()) {
      store.searchProjects(query);
    } else {
      // Recargar vista actual según el modo (v0.4.0)
      if (store.viewMode() === 'groups') {
        store.loadRootProjects();
      } else if (store.currentGroup()) {
        store.loadSubprojects(store.currentGroup()!.id);
      }
    }
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
    const confirmed = await confirm(`¿Eliminar proyecto "${project.name}"?`, {
      title: 'Confirmar eliminación',
      kind: 'warning',
    });

    if (confirmed) {
      try {
        await store.deleteProject(project.id);
      } catch {
        alert('Error al eliminar el proyecto');
      }
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
    console.log('🔧 [APP] handleFormSubmit iniciado con datos:', data);
    console.log('🔧 [APP] editingProject:', editingProject());
    try {
      if (editingProject()) {
        console.log('🔧 [APP] Llamando a store.updateProject...');
        await store.updateProject(editingProject()!.id, data);
        console.log('✅ [APP] store.updateProject exitoso');
      } else {
        console.log('🔧 [APP] Llamando a store.createProject...');
        await store.createProject(data);
        console.log('✅ [APP] store.createProject exitoso');
      }
      console.log('🔧 [APP] Cerrando formulario...');
      setShowForm(false);
      setEditingProject(null);
      console.log('✅ [APP] Formulario cerrado exitosamente');
    } catch (err) {
      console.error('❌ [APP] Error en handleFormSubmit:', err);
      alert(
        'Error al guardar el proyecto: ' +
          (err instanceof Error ? err.message : 'Error desconocido')
      );
    }
  };

  const handleFormCancel = () => {
    setShowForm(false);
    setEditingProject(null);
  };

  return (
    <div class="min-h-screen bg-gray-50 transition-colors duration-300 dark:bg-gray-900">
      <Toaster position="top-right" />
      <ThemeToggle />

      {/* Header */}
      <header class="border-b border-gray-200 bg-white shadow-sm dark:border-gray-700 dark:bg-gray-800">
        <div class="px-3 py-2 sm:px-4 lg:px-5">
          {/* Primera fila: Título y botones principales */}
          <div class="flex items-center justify-between">
            <h1 class="text-xl font-bold text-gray-900 dark:text-white sm:text-2xl">
              Gestor de Proyectos
            </h1>
            <div class="flex gap-2">
              <button
                onClick={() => setShowTreeView(!showTreeView())}
                class="rounded-lg bg-teal-600 px-3 py-1.5 text-sm text-white hover:bg-teal-700 focus:outline-none focus:ring-2 focus:ring-teal-500 focus:ring-offset-2 dark:bg-teal-500 dark:hover:bg-teal-600"
              >
                🌳 {showTreeView() ? 'Ocultar' : 'Árbol'}
              </button>
              <button
                onClick={() => setShowSettings(true)}
                class="rounded-lg bg-gray-600 px-3 py-1.5 text-sm text-white hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2 dark:bg-gray-500 dark:hover:bg-gray-600"
              >
                ⚙️ Configuración
              </button>
              <button
                onClick={() => setShowAnalytics(!showAnalytics())}
                class="rounded-lg bg-purple-600 px-3 py-1.5 text-sm text-white hover:bg-purple-700 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 dark:bg-purple-500 dark:hover:bg-purple-600"
              >
                📊 {showAnalytics() ? 'Proyectos' : 'Estadísticas'}
              </button>
              <button
                onClick={handleNewProject}
                class="rounded-lg bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:bg-blue-500 dark:hover:bg-blue-600"
              >
                + Nuevo Proyecto
              </button>
            </div>
          </div>

          {/* Segunda fila: Navegación + Búsqueda y Filtros (solo en vista de proyectos) */}
          <Show when={!showAnalytics()}>
            {/* Breadcrumb de navegación (v0.4.0) */}
            <Show
              when={store.viewMode() === 'subprojects' && store.currentGroup()}
            >
              <div class="mt-2 flex items-center gap-2">
                <button
                  onClick={() => store.navigateBack()}
                  class="rounded-lg bg-gray-100 px-3 py-1 text-sm text-gray-700 hover:bg-gray-200 dark:bg-gray-700 dark:text-gray-300 dark:hover:bg-gray-600"
                >
                  ← Volver a Grupos
                </button>
                <span class="text-sm text-gray-600 dark:text-gray-400">
                  / 📁 {store.currentGroup()?.name}
                </span>
              </div>
            </Show>

            <div class="mt-2 flex flex-wrap items-center gap-2">
              <div class="min-w-[200px] flex-1">
                <SearchBar onSearch={handleSearch} value={searchQuery()} />
              </div>
              <Show when={filterProps()}>
                <ProjectFilters {...filterProps()!} />
              </Show>
            </div>
          </Show>
        </div>
      </header>

      {/* Main Content */}
      <main class="px-2 py-4 sm:px-3 lg:px-4">
        {/* Analytics View */}
        <Show when={showAnalytics()}>
          <Analytics />
        </Show>

        {/* Projects View */}
        <Show when={!showAnalytics()}>
          <div class="flex gap-4">
            {/* TreeView Sidebar - Solo visible cuando showTreeView es true */}
            <Show when={showTreeView()}>
              <div class="w-80 flex-shrink-0">
                <TreeView
                  onSelectProject={(project) => {
                    // Si es un grupo, navegar a sus subproyectos
                    if (!project.parent_id) {
                      store.navigateToGroup(project);
                    } else {
                      // Si es un subproyecto, editarlo
                      handleEdit(project);
                    }
                  }}
                />
              </div>
            </Show>

            {/* Main Content Area */}
            <div class="flex-1">
              {/* Error Message */}
              <Show when={store.error()}>
                <div class="mb-4 rounded-lg bg-red-50 p-4 text-red-800 dark:bg-red-900/20 dark:text-red-200">
                  <p class="font-medium">Error:</p>
                  <p>{store.error()}</p>
                </div>
              </Show>

              {/* Loading State */}
              <Show when={store.isLoading()}>
                <div class="py-12 text-center">
                  <p class="text-gray-600 dark:text-gray-400">
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
                    // Recargar la vista actual (grupos o subproyectos)
                    if (store.viewMode() === 'groups') {
                      store.loadRootProjects();
                    } else if (store.currentGroup()) {
                      store.loadSubprojects(store.currentGroup()!.id);
                    }
                  }}
                  renderFilters={setFilterProps}
                  viewMode={store.viewMode()}
                  onViewGroup={(group) => store.navigateToGroup(group)}
                  searchActive={searchQuery().trim().length > 0}
                />
              </Show>
            </div>
          </div>
        </Show>
      </main>

      {/* Modal Form */}
      <Show when={showForm()}>
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 p-4 dark:bg-opacity-70">
          <div class="max-h-[90vh] w-full max-w-lg overflow-y-auto rounded-lg bg-white p-6 shadow-xl dark:bg-gray-800">
            <h2 class="mb-4 text-xl font-semibold text-gray-900 dark:text-white">
              {editingProject() ? 'Editar Proyecto' : 'Nuevo Proyecto'}
            </h2>
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
    </div>
  );
};

// Componente principal envuelto con providers
const App: Component = () => {
  return (
    <ThemeProvider>
      <ShortcutsProvider>
        <AppContent />
      </ShortcutsProvider>
    </ThemeProvider>
  );
};

export default App;
