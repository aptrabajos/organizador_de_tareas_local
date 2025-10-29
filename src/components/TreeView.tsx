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

const TreeView: Component<TreeViewProps> = (props) => {
  const [treeData, setTreeData] = createSignal<TreeNode[]>([]);
  const [loading, setLoading] = createSignal(true);

  onMount(async () => {
    await loadTree();
  });

  const loadTree = async () => {
    setLoading(true);
    try {
      const roots = await getRootProjects();
      const tree: TreeNode[] = [];

      for (const root of roots) {
        const children = await getSubprojects(root.id);
        tree.push({
          ...root,
          children: children.map((child) => ({ ...child, children: [] })),
          expanded: false,
        });
      }

      setTreeData(tree);
    } catch (err) {
      console.error('Error loading tree:', err);
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
    const indent = level * 24; // 24px por nivel

    return (
      <div>
        <div
          class="group flex cursor-pointer items-center gap-2 rounded-lg py-2 pl-2 pr-2 hover:bg-gray-100 dark:hover:bg-gray-700"
          style={{ 'padding-left': `${indent + 8}px` }}
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
              <span class="text-gray-500">
                {node.expanded ? '▼' : '▶'}
              </span>
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
      </Show>
    </div>
  );
};

export default TreeView;
