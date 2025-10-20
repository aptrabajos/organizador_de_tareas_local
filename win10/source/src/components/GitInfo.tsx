import { Component, createSignal, onMount, Show } from 'solid-js';
import { getGitBranch, getGitStatus } from '../services/api';

interface GitInfoProps {
  projectPath: string;
}

const GitInfo: Component<GitInfoProps> = (props) => {
  const [branch, setBranch] = createSignal<string | null>(null);
  const [hasChanges, setHasChanges] = createSignal(false);
  const [isGitRepo, setIsGitRepo] = createSignal(false);

  onMount(async () => {
    try {
      const branchName = await getGitBranch(props.projectPath);
      setBranch(branchName);
      setIsGitRepo(true);

      // Check if there are changes
      const status = await getGitStatus(props.projectPath);
      setHasChanges(status.trim().length > 0);
    } catch {
      // Not a git repository or error
      setIsGitRepo(false);
    }
  });

  return (
    <Show when={isGitRepo()}>
      <div class="flex items-center space-x-2 text-sm">
        {/* Branch indicator */}
        <div class="flex items-center space-x-1 rounded bg-gray-100 px-2 py-1 dark:bg-gray-700">
          <svg
            class="h-4 w-4 text-gray-600 dark:text-gray-300"
            fill="currentColor"
            viewBox="0 0 16 16"
          >
            <path
              fill-rule="evenodd"
              d="M11.75 2.5a.75.75 0 100 1.5.75.75 0 000-1.5zm-2.25.75a2.25 2.25 0 113 2.122V6A2.5 2.5 0 0110 8.5H6a1 1 0 00-1 1v1.128a2.251 2.251 0 11-1.5 0V5.372a2.25 2.25 0 111.5 0v1.836A2.492 2.492 0 016 7h4a1 1 0 001-1v-.628A2.25 2.25 0 019.5 3.25zM4.25 12a.75.75 0 100 1.5.75.75 0 000-1.5zM3.5 3.25a.75.75 0 111.5 0 .75.75 0 01-1.5 0z"
            />
          </svg>
          <span class="font-mono text-xs font-medium text-gray-700 dark:text-gray-200">
            {branch()}
          </span>
        </div>

        {/* Changes indicator */}
        <Show when={hasChanges()}>
          <div
            class="flex items-center space-x-1 rounded bg-yellow-100 px-2 py-1 dark:bg-yellow-900/30"
            title="Hay cambios sin commitear"
          >
            <svg
              class="h-3 w-3 text-yellow-600 dark:text-yellow-400"
              fill="currentColor"
              viewBox="0 0 16 16"
            >
              <circle cx="8" cy="8" r="6" />
            </svg>
            <span class="text-xs font-medium text-yellow-700 dark:text-yellow-400">
              Changes
            </span>
          </div>
        </Show>
      </div>
    </Show>
  );
};

export default GitInfo;
