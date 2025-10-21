import { Component, createSignal, Show } from 'solid-js';
import toast from 'solid-toast';
import { gitCommit, gitPush, getGitModifiedFiles } from '../services/api';

interface GitCommitModalProps {
  projectPath: string;
  onClose: () => void;
  onSuccess?: () => void;
}

const GitCommitModal: Component<GitCommitModalProps> = (props) => {
  const [message, setMessage] = createSignal('');
  const [pushAfterCommit, setPushAfterCommit] = createSignal(false);
  const [isLoading, setIsLoading] = createSignal(false);
  const [stagedFiles, setStagedFiles] = createSignal<string[]>([]);

  // Cargar archivos staged al montar
  const loadStagedFiles = async () => {
    try {
      const files = await getGitModifiedFiles(props.projectPath);
      setStagedFiles(files);
    } catch (error) {
      console.error('Error loading files:', error);
    }
  };

  loadStagedFiles();

  const handleSubmit = async (e: Event) => {
    e.preventDefault();

    const msg = message().trim();
    if (msg.length < 3) {
      toast.error('El mensaje debe tener al menos 3 caracteres');
      return;
    }

    setIsLoading(true);
    try {
      // Crear commit
      await gitCommit(props.projectPath, msg);
      toast.success('✅ Commit creado exitosamente');

      // Push si está habilitado
      if (pushAfterCommit()) {
        try {
          await gitPush(props.projectPath);
          toast.success('✅ Push exitoso');
        } catch (pushError) {
          toast.error(`Error en push: ${pushError}`);
        }
      }

      props.onSuccess?.();
      props.onClose();
    } catch (error) {
      toast.error(`Error: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 p-4 dark:bg-opacity-70">
      <div class="w-full max-w-md rounded-lg bg-white p-6 shadow-xl dark:bg-gray-800">
        {/* Header */}
        <div class="mb-4 flex items-center justify-between">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
            💾 Crear Commit
          </h2>
          <button
            onClick={props.onClose}
            class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
          >
            ✕
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} class="space-y-4">
          {/* Mensaje del commit */}
          <div>
            <label class="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-300">
              Mensaje del Commit
            </label>
            <textarea
              value={message()}
              onInput={(e) => setMessage(e.currentTarget.value)}
              placeholder="ej: feat: nueva funcionalidad de atajos de teclado"
              rows={3}
              class="w-full rounded-md border border-gray-300 px-3 py-2 text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
              required
            />
            <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
              Mínimo 3 caracteres. Usa formato convencional: feat:, fix:,
              docs:, etc.
            </p>
          </div>

          {/* Archivos staged */}
          <Show when={stagedFiles().length > 0}>
            <div class="rounded-lg border border-gray-200 bg-gray-50 p-3 dark:border-gray-700 dark:bg-gray-900">
              <p class="mb-2 text-sm font-medium text-gray-700 dark:text-gray-300">
                Archivos incluidos ({stagedFiles().length}):
              </p>
              <div class="max-h-32 space-y-1 overflow-y-auto">
                {stagedFiles().map((file) => (
                  <div class="flex items-center gap-2">
                    <span class="text-green-600 dark:text-green-400">✓</span>
                    <code class="text-xs text-gray-600 dark:text-gray-400">
                      {file}
                    </code>
                  </div>
                ))}
              </div>
            </div>
          </Show>

          {/* Push después de commit */}
          <div class="flex items-center justify-between rounded-lg border border-gray-200 bg-gray-50 p-3 dark:border-gray-700 dark:bg-gray-900">
            <div>
              <p class="font-medium text-gray-900 dark:text-white">
                Push automático
              </p>
              <p class="text-sm text-gray-600 dark:text-gray-400">
                Hacer push después de crear el commit
              </p>
            </div>
            <label class="relative inline-flex cursor-pointer items-center">
              <input
                type="checkbox"
                class="peer sr-only"
                checked={pushAfterCommit()}
                onChange={(e) => setPushAfterCommit(e.currentTarget.checked)}
              />
              <div class="peer h-6 w-11 rounded-full bg-gray-200 after:absolute after:start-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:border-gray-600 dark:bg-gray-700 dark:peer-focus:ring-blue-800 rtl:peer-checked:after:-translate-x-full" />
            </label>
          </div>

          {/* Buttons */}
          <div class="flex gap-3">
            <button
              type="button"
              onClick={props.onClose}
              class="flex-1 rounded-md border border-gray-300 px-4 py-2 text-gray-700 hover:bg-gray-50 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
            >
              Cancelar
            </button>
            <button
              type="submit"
              disabled={isLoading() || message().trim().length < 3}
              class="flex-1 rounded-md bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:opacity-50 dark:bg-blue-500 dark:hover:bg-blue-600"
            >
              {isLoading() ? 'Creando...' : '✓ Commit'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default GitCommitModal;
