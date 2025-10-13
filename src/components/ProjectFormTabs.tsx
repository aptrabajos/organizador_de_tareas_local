import { Component, createSignal, Show } from 'solid-js';
import type { Project } from '../types/project';
import ProjectForm, { type ProjectFormData } from './ProjectForm';
import ProjectLinks from './ProjectLinks';
import AttachmentManager from './AttachmentManager';

interface ProjectFormTabsProps {
  project?: Project;
  onSubmit: (data: ProjectFormData) => void;
  onCancel: () => void;
}

const ProjectFormTabs: Component<ProjectFormTabsProps> = (props) => {
  const [activeTab, setActiveTab] = createSignal<
    'details' | 'links' | 'attachments'
  >('details');

  return (
    <div class="space-y-4">
      {/* Pestañas */}
      <div class="border-b border-gray-200 dark:border-gray-700">
        <nav class="-mb-px flex space-x-8">
          <button
            onClick={() => setActiveTab('details')}
            class={`border-b-2 px-1 py-2 text-sm font-medium ${
              activeTab() === 'details'
                ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400 dark:hover:border-gray-600 dark:hover:text-gray-300'
            }`}
          >
            📝 Detalles
          </button>
          <Show when={props.project?.id}>
            <button
              onClick={() => setActiveTab('links')}
              class={`border-b-2 px-1 py-2 text-sm font-medium ${
                activeTab() === 'links'
                  ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                  : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400 dark:hover:border-gray-600 dark:hover:text-gray-300'
              }`}
            >
              🔗 Enlaces
            </button>
            <button
              onClick={() => setActiveTab('attachments')}
              class={`border-b-2 px-1 py-2 text-sm font-medium ${
                activeTab() === 'attachments'
                  ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                  : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400 dark:hover:border-gray-600 dark:hover:text-gray-300'
              }`}
            >
              📎 Archivos
            </button>
          </Show>
        </nav>
      </div>

      {/* Contenido de las pestañas */}
      <Show when={activeTab() === 'details'}>
        <ProjectForm
          project={props.project}
          onSubmit={props.onSubmit}
          onCancel={props.onCancel}
        />
      </Show>

      <Show when={activeTab() === 'links' && props.project?.id}>
        <ProjectLinks
          projectId={props.project!.id}
          projectName={props.project!.name}
        />
      </Show>

      <Show when={activeTab() === 'attachments' && props.project?.id}>
        <AttachmentManager projectId={props.project!.id} />
      </Show>
    </div>
  );
};

export default ProjectFormTabs;
