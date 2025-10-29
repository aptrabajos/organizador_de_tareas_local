import { Component, createSignal, For, Show, onMount } from 'solid-js';
import type { Project } from '../types/project';
import { getRootProjects, getSubprojects } from '../services/api';

interface TreeViewProps {
  onSelectProject: (project: Project) => void;
}

interface TreeNode extends Project {
  children?: TreeNode[];
  expanded?: boolean;
}

// Constantes para evitar magic numbers
const INDENT_PX_PER_LEVEL = 24;
const BASE_PADDING_PX = 8;

// Utilidad para mapear proyectos a TreeNodes
const mapToTreeNode = (
  project: Project,
  children: Project[] = []
): TreeNode => ({
  ...project,
  children: children.map((child) => ({ ...child, children: [] })),
  expanded: false,
});

const TreeView: Component<TreeViewProps> = (props) => {
  const [treeData, setTreeData] = createSignal<TreeNode[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    await loadTree();
  });

  const loadTree = async () => {
    setLoading(true);
    setError(null);
    try {
      const roots = await getRootProjects();

      // Performance: Cargar todos los subproyectos en paralelo con Promise.all
      const treePromises = roots.map(async (root) => {
        const children = await getSubprojects(root.id);
        return mapToTreeNode(root, children);
      });

      const tree = await Promise.all(treePromises);
      setTreeData(tree);
    } catch (err) {
      console.error('Error loading tree:', err);
      setError('Error al cargar el árbol de proyectos');
    } finally {
      setLoading(false);
    }
  };

  const toggleExpanded = (projectId: number) => {
    setTreeData((prev) =>
      prev.map((node) => {
        if (node.id === projectId) {
          return { ...node, expanded: !node.expanded };
        }
        return node;
      })
    );
  };

  const renderNode = (node: TreeNode, level: number = 0) => {
    const hasChildren = node.children && node.children.length > 0;
    const indent = level * INDENT_PX_PER_LEVEL;

    return (
      <div>
        <div
          class="group flex cursor-pointer items-center gap-2 rounded-lg py-2 pl-2 pr-2 hover:bg-gray-100 dark:hover:bg-gray-700"
          style={{ 'padding-left': `${indent + BASE_PADDING_PX}px` }}
          onClick={() => {
            if (hasChildren) {
              toggleExpanded(node.id);
            }
            props.onSelectProject(node);
          }}
        >
          {/* Indicador de expansión */}
          <div class="w-4 flex-shrink-0">
            {hasChildren ? (
              <span class="text-gray-500">{node.expanded ? '▼' : '▶'}</span>
            ) : (
              <span class="text-gray-300">•</span>
            )}
          </div>

          {/* Ícono */}
          <Show
            when={node.image_data}
            fallback={
              <span class="text-xl">
                {node.group_icon || (hasChildren ? '📁' : '📄')}
              </span>
            }
          >
            <img
              src={node.image_data}
              alt={node.name}
              class="h-8 w-8 flex-shrink-0 rounded object-cover"
            />
          </Show>

          {/* Nombre */}
          <div class="flex-1 truncate">
            <p class="truncate text-sm font-medium text-gray-900 dark:text-white">
              {node.name}
            </p>
            <Show when={hasChildren}>
              <p class="text-xs text-gray-500 dark:text-gray-400">
                {node.children!.length}{' '}
                {node.children!.length === 1 ? 'subproyecto' : 'subproyectos'}
              </p>
            </Show>
          </div>

          {/* Badge de estado */}
          <Show when={node.status}>
            <span
              class={`rounded-full px-2 py-0.5 text-xs ${
                node.status === 'activo'
                  ? 'bg-green-100 text-green-700'
                  : node.status === 'pausado'
                    ? 'bg-yellow-100 text-yellow-700'
                    : 'bg-gray-100 text-gray-700'
              }`}
            >
              {node.status}
            </span>
          </Show>
        </div>

        {/* Subproyectos (recursivo) */}
        <Show when={node.expanded && hasChildren}>
          <For each={node.children}>
            {(child) => renderNode(child, level + 1)}
          </For>
        </Show>
      </div>
    );
  };

  return (
    <div class="rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800">
      <h2 class="mb-4 text-lg font-semibold text-gray-900 dark:text-white">
        🌳 Vista de Árbol
      </h2>

      <Show
        when={!loading()}
        fallback={
          <p class="text-center text-gray-500 dark:text-gray-400">
            Cargando árbol...
          </p>
        }
      >
        {/* Mostrar error si existe */}
        <Show
          when={error()}
          fallback={
            <Show
              when={treeData().length > 0}
              fallback={
                <p class="text-center text-gray-500 dark:text-gray-400">
                  No hay proyectos
                </p>
              }
            >
              <div class="space-y-1">
                <For each={treeData()}>{(node) => renderNode(node)}</For>
              </div>
            </Show>
          }
        >
          <div class="rounded-lg bg-red-50 p-3 text-center dark:bg-red-900/20">
            <p class="text-sm text-red-700 dark:text-red-300">{error()}</p>
            <button
              onClick={loadTree}
              class="mt-2 text-xs text-red-600 underline hover:text-red-800 dark:text-red-400 dark:hover:text-red-200"
            >
              Reintentar
            </button>
          </div>
        </Show>
      </Show>
    </div>
  );
};

export default TreeView;
