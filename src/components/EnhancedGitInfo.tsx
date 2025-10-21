import {
  Component,
  createSignal,
  onMount,
  Show,
  For,
  createEffect,
} from 'solid-js';
import toast from 'solid-toast';
import type { GitCommit, GitFileCount } from '../types/git';
import {
  getGitBranch,
  getGitFileCount,
  getGitModifiedFiles,
  getRecentCommits,
  getGitRemoteUrl,
  getGitAheadBehind,
  gitAdd,
  gitPush,
  gitPull,
} from '../services/api';

interface EnhancedGitInfoProps {
  projectPath: string;
  onCommitClick?: () => void;
}

const EnhancedGitInfo: Component<EnhancedGitInfoProps> = (props) => {
  const [branch, setBranch] = createSignal<string | null>(null);
  const [fileCount, setFileCount] = createSignal<GitFileCount | null>(null);
  const [commits, setCommits] = createSignal<GitCommit[]>([]);
  const [remoteUrl, setRemoteUrl] = createSignal<string | null>(null);
  const [ahead, setAhead] = createSignal(0);
  const [behind, setBehind] = createSignal(0);
  const [isGitRepo, setIsGitRepo] = createSignal(false);
  const [expanded, setExpanded] = createSignal(false);
  const [isLoading, setIsLoading] = createSignal(false);

  const loadGitInfo = async () => {
    try {
      const branchName = await getGitBranch(props.projectPath);
      setBranch(branchName);
      setIsGitRepo(true);

      // Cargar información en paralelo
      const [count, commitsData, url, [aheadCount, behindCount]] =
        await Promise.all([
          getGitFileCount(props.projectPath).catch(() => null),
          getRecentCommits(props.projectPath, 5).catch(() => []),
          getGitRemoteUrl(props.projectPath).catch(() => null),
          getGitAheadBehind(props.projectPath).catch(() => [0, 0] as [
            number,
            number
          ]),
        ]);

      setFileCount(count);
      setCommits(commitsData);
      setRemoteUrl(url);
      setAhead(aheadCount);
      setBehind(behindCount);
    } catch {
      setIsGitRepo(false);
    }
  };

  onMount(() => {
    loadGitInfo();
  });

  const handleStageAll = async () => {
    setIsLoading(true);
    try {
      const files = await getGitModifiedFiles(props.projectPath);
      if (files.length === 0) {
        toast('No hay archivos para agregar', { icon: '⚠️' });
        return;
      }

      await gitAdd(props.projectPath, files);
      toast.success(`✅ ${files.length} archivos staged`);
      await loadGitInfo();
    } catch (error) {
      toast.error(`Error: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handlePush = async () => {
    if (!window.confirm('¿Hacer push de los commits locales?')) return;

    setIsLoading(true);
    try {
      const result = await gitPush(props.projectPath);
      toast.success('✅ Push exitoso');
      console.log(result);
      await loadGitInfo();
    } catch (error) {
      toast.error(`Error en push: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handlePull = async () => {
    if (!window.confirm('¿Hacer pull del repositorio remoto?')) return;

    setIsLoading(true);
    try {
      const result = await gitPull(props.projectPath);
      toast.success('✅ Pull exitoso');
      console.log(result);
      await loadGitInfo();
    } catch (error) {
      toast.error(`Error en pull: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const getGitHubUrl = () => {
    const url = remoteUrl();
    if (!url) return null;

    // Convertir URLs SSH/HTTPS a URL web
    if (url.includes('github.com')) {
      return url
        .replace('git@github.com:', 'https://github.com/')
        .replace('.git', '');
    }
    if (url.includes('gitlab.com')) {
      return url
        .replace('git@gitlab.com:', 'https://gitlab.com/')
        .replace('.git', '');
    }
    return null;
  };

  return (
    <Show when={isGitRepo()}>
      <div class="space-y-2 rounded-lg border border-gray-200 bg-gray-50 p-3 dark:border-gray-700 dark:bg-gray-800">
        {/* Header compacto */}
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            {/* Branch */}
            <div class="flex items-center gap-1 rounded bg-blue-100 px-2 py-0.5 dark:bg-blue-900/30">
              <svg
                class="h-3 w-3 text-blue-600 dark:text-blue-400"
                fill="currentColor"
                viewBox="0 0 16 16"
              >
                <path
                  fill-rule="evenodd"
                  d="M11.75 2.5a.75.75 0 100 1.5.75.75 0 000-1.5zm-2.25.75a2.25 2.25 0 113 2.122V6A2.5 2.5 0 0110 8.5H6a1 1 0 00-1 1v1.128a2.251 2.251 0 11-1.5 0V5.372a2.25 2.25 0 111.5 0v1.836A2.492 2.492 0 016 7h4a1 1 0 001-1v-.628A2.25 2.25 0 019.5 3.25zM4.25 12a.75.75 0 100 1.5.75.75 0 000-1.5zM3.5 3.25a.75.75 0 111.5 0 .75.75 0 01-1.5 0z"
                />
              </svg>
              <span class="text-xs font-mono font-medium text-blue-700 dark:text-blue-300">
                {branch()}
              </span>
            </div>

            {/* File count */}
            <Show when={fileCount()}>
              {(count) => (
                <Show when={count().modified + count().staged > 0}>
                  <div class="flex items-center gap-1 rounded bg-orange-100 px-2 py-0.5 dark:bg-orange-900/30">
                    <span class="text-xs font-medium text-orange-700 dark:text-orange-300">
                      {count().staged > 0 && `${count().staged} staged`}
                      {count().staged > 0 && count().modified > 0 && ', '}
                      {count().modified > 0 && `${count().modified} modified`}
                    </span>
                  </div>
                </Show>
              )}
            </Show>

            {/* Ahead/Behind */}
            <Show when={ahead() > 0 || behind() > 0}>
              <div class="flex items-center gap-1 rounded bg-purple-100 px-2 py-0.5 dark:bg-purple-900/30">
                <span class="text-xs font-medium text-purple-700 dark:text-purple-300">
                  {ahead() > 0 && `↑${ahead()}`}
                  {ahead() > 0 && behind() > 0 && ' '}
                  {behind() > 0 && `↓${behind()}`}
                </span>
              </div>
            </Show>
          </div>

          {/* Toggle expand */}
          <button
            onClick={() => setExpanded((prev) => !prev)}
            class="text-xs text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300"
          >
            {expanded() ? '▼ Menos' : '▶ Más'}
          </button>
        </div>

        {/* Expanded content */}
        <Show when={expanded()}>
          <div class="space-y-2 border-t border-gray-200 pt-2 dark:border-gray-700">
            {/* Action buttons */}
            <div class="flex flex-wrap gap-1">
              <button
                onClick={handleStageAll}
                disabled={isLoading()}
                class="rounded bg-green-600 px-2 py-1 text-xs text-white hover:bg-green-700 disabled:opacity-50 dark:bg-green-500 dark:hover:bg-green-600"
              >
                📝 Stage All
              </button>
              <button
                onClick={props.onCommitClick}
                class="rounded bg-blue-600 px-2 py-1 text-xs text-white hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600"
              >
                💾 Commit
              </button>
              <button
                onClick={handlePush}
                disabled={isLoading() || ahead() === 0}
                class="rounded bg-purple-600 px-2 py-1 text-xs text-white hover:bg-purple-700 disabled:opacity-50 dark:bg-purple-500 dark:hover:bg-purple-600"
              >
                🚀 Push
              </button>
              <button
                onClick={handlePull}
                disabled={isLoading()}
                class="rounded bg-cyan-600 px-2 py-1 text-xs text-white hover:bg-cyan-700 disabled:opacity-50 dark:bg-cyan-500 dark:hover:bg-cyan-600"
              >
                ⬇️ Pull
              </button>
              <Show when={getGitHubUrl()}>
                <a
                  href={getGitHubUrl()!}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="rounded bg-gray-700 px-2 py-1 text-xs text-white hover:bg-gray-800 dark:bg-gray-600 dark:hover:bg-gray-700"
                >
                  🔗 Repo
                </a>
              </Show>
            </div>

            {/* Recent commits */}
            <Show when={commits().length > 0}>
              <div class="space-y-1">
                <p class="text-xs font-semibold text-gray-700 dark:text-gray-300">
                  Últimos commits:
                </p>
                <div class="max-h-40 space-y-1 overflow-y-auto">
                  <For each={commits()}>
                    {(commit) => (
                      <div class="rounded bg-white p-2 dark:bg-gray-700">
                        <div class="flex items-start justify-between gap-2">
                          <div class="min-w-0 flex-1">
                            <p class="truncate text-xs font-medium text-gray-900 dark:text-white">
                              {commit.message}
                            </p>
                            <p class="text-xs text-gray-500 dark:text-gray-400">
                              {commit.author} • {commit.date}
                            </p>
                          </div>
                          <code class="flex-shrink-0 rounded bg-gray-100 px-1 text-xs text-gray-600 dark:bg-gray-600 dark:text-gray-300">
                            {commit.hash}
                          </code>
                        </div>
                      </div>
                    )}
                  </For>
                </div>
              </div>
            </Show>
          </div>
        </Show>
      </div>
    </Show>
  );
};

export default EnhancedGitInfo;
