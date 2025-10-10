import { Component, createSignal, onMount, Show } from 'solid-js';
import { createProjectStore } from './stores/projectStore';
import SearchBar from './components/SearchBar';
import ProjectList from './components/ProjectList';
import ProjectForm, { type ProjectFormData } from './components/ProjectForm';
import type { Project } from './types/project';

const App: Component = () => {
  const store = createProjectStore();
  const [searchQuery, setSearchQuery] = createSignal('');
  const [showForm, setShowForm] = createSignal(false);
  const [editingProject, setEditingProject] = createSignal<Project | null>(
    null
  );

  onMount(() => {
    store.loadProjects();
  });

  const handleSearch = (query: string) => {
    setSearchQuery(query);
    if (query.trim()) {
      store.searchProjects(query);
    } else {
      store.loadProjects();
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
    if (confirm(`¿Eliminar proyecto "${project.name}"?`)) {
      try {
        await store.deleteProject(project.id);
      } catch (_err) {
        alert('Error al eliminar el proyecto');
      }
    }
  };

  const handleOpenTerminal = async (project: Project) => {
    try {
      await store.openTerminal(project.local_path);
    } catch (_err) {
      alert('Error al abrir terminal');
    }
  };

  const handleFormSubmit = async (data: ProjectFormData) => {
    try {
      if (editingProject()) {
        await store.updateProject(editingProject()!.id, data);
      } else {
        await store.createProject(data);
      }
      setShowForm(false);
      setEditingProject(null);
    } catch (_err) {
      alert('Error al guardar el proyecto');
    }
  };

  const handleFormCancel = () => {
    setShowForm(false);
    setEditingProject(null);
  };

  return (
    <div class="min-h-screen bg-gray-50">
      {/* Header */}
      <header class="border-b border-gray-200 bg-white shadow-sm">
        <div class="mx-auto max-w-7xl px-4 py-4 sm:px-6 lg:px-8">
          <div class="flex items-center justify-between">
            <h1 class="text-2xl font-bold text-gray-900">
              Gestor de Proyectos
            </h1>
            <button
              onClick={handleNewProject}
              class="rounded-lg bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
            >
              + Nuevo Proyecto
            </button>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main class="mx-auto max-w-7xl px-4 py-6 sm:px-6 lg:px-8">
        {/* Search Bar */}
        <div class="mb-6">
          <SearchBar onSearch={handleSearch} value={searchQuery()} />
        </div>

        {/* Error Message */}
        <Show when={store.error()}>
          <div class="mb-4 rounded-lg bg-red-50 p-4 text-red-800">
            <p class="font-medium">Error:</p>
            <p>{store.error()}</p>
          </div>
        </Show>

        {/* Loading State */}
        <Show when={store.isLoading()}>
          <div class="py-12 text-center">
            <p class="text-gray-600">Cargando proyectos...</p>
          </div>
        </Show>

        {/* Project List */}
        <Show when={!store.isLoading()}>
          <ProjectList
            projects={store.projects()}
            onEdit={handleEdit}
            onDelete={handleDelete}
            onOpenTerminal={handleOpenTerminal}
          />
        </Show>
      </main>

      {/* Modal Form */}
      <Show when={showForm()}>
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 p-4">
          <div class="max-h-[90vh] w-full max-w-lg overflow-y-auto rounded-lg bg-white p-6 shadow-xl">
            <h2 class="mb-4 text-xl font-semibold text-gray-900">
              {editingProject() ? 'Editar Proyecto' : 'Nuevo Proyecto'}
            </h2>
            <ProjectForm
              project={editingProject() || undefined}
              onSubmit={handleFormSubmit}
              onCancel={handleFormCancel}
            />
          </div>
        </div>
      </Show>
    </div>
  );
};

export default App;
