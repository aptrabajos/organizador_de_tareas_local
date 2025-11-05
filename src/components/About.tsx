import { Component } from 'solid-js';

interface AboutProps {
  onClose: () => void;
}

const About: Component<AboutProps> = (props) => {
  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 p-4 dark:bg-opacity-70">
      <div class="max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-lg bg-white p-6 shadow-xl dark:bg-gray-800">
        {/* Header */}
        <div class="mb-6 flex items-center justify-between border-b border-gray-200 pb-4 dark:border-gray-700">
          <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
            📖 Acerca de
          </h2>
          <button
            onClick={() => props.onClose()}
            class="rounded-lg p-2 text-gray-500 hover:bg-gray-100 hover:text-gray-700 dark:hover:bg-gray-700 dark:hover:text-gray-300"
          >
            ✕
          </button>
        </div>

        {/* Content */}
        <div class="space-y-6">
          {/* App Info */}
          <div>
            <h3 class="mb-2 text-xl font-semibold text-gray-900 dark:text-white">
              Gestor de Proyectos
            </h3>
            <p class="text-sm text-gray-600 dark:text-gray-400">
              Versión: <span class="font-mono font-semibold">0.4.3</span>
            </p>
            <p class="mt-2 text-gray-700 dark:text-gray-300">
              Gestor de proyectos multiplataforma con código optimizado y 100%
              tipado
            </p>
          </div>

          {/* Features */}
          <div>
            <h4 class="mb-3 font-semibold text-gray-900 dark:text-white">
              ✨ Características Principales
            </h4>
            <ul class="space-y-2 text-sm text-gray-700 dark:text-gray-300">
              <li class="flex items-start gap-2">
                <span class="text-blue-600 dark:text-blue-400">•</span>
                <span>
                  Gestión completa de proyectos con grupos jerárquicos
                </span>
              </li>
              <li class="flex items-start gap-2">
                <span class="text-blue-600 dark:text-blue-400">•</span>
                <span>Editor Markdown con preview en tiempo real</span>
              </li>
              <li class="flex items-start gap-2">
                <span class="text-blue-600 dark:text-blue-400">•</span>
                <span>Sistema de diario (journal) y TODOs por proyecto</span>
              </li>
              <li class="flex items-start gap-2">
                <span class="text-blue-600 dark:text-blue-400">•</span>
                <span>Integración Git con commits visuales</span>
              </li>
              <li class="flex items-start gap-2">
                <span class="text-blue-600 dark:text-blue-400">•</span>
                <span>Exportación de proyectos a PDF</span>
              </li>
              <li class="flex items-start gap-2">
                <span class="text-blue-600 dark:text-blue-400">•</span>
                <span>Atajos de teclado configurables</span>
              </li>
              <li class="flex items-start gap-2">
                <span class="text-blue-600 dark:text-blue-400">•</span>
                <span>Drag & Drop para organización visual</span>
              </li>
              <li class="flex items-start gap-2">
                <span class="text-blue-600 dark:text-blue-400">•</span>
                <span>Dark mode y temas personalizables</span>
              </li>
            </ul>
          </div>

          {/* Tech Stack */}
          <div>
            <h4 class="mb-3 font-semibold text-gray-900 dark:text-white">
              🛠️ Tecnologías
            </h4>
            <div class="grid grid-cols-2 gap-3 text-sm">
              <div class="rounded-lg bg-gray-50 p-3 dark:bg-gray-700">
                <p class="font-semibold text-gray-900 dark:text-white">
                  Frontend
                </p>
                <ul class="mt-1 space-y-1 text-gray-600 dark:text-gray-400">
                  <li>• SolidJS 1.9.3</li>
                  <li>• TypeScript 5.7.3</li>
                  <li>• TailwindCSS 3.4.17</li>
                  <li>• Vite 6.0.5</li>
                </ul>
              </div>
              <div class="rounded-lg bg-gray-50 p-3 dark:bg-gray-700">
                <p class="font-semibold text-gray-900 dark:text-white">
                  Backend
                </p>
                <ul class="mt-1 space-y-1 text-gray-600 dark:text-gray-400">
                  <li>• Rust + Tauri 2.1.0</li>
                  <li>• SQLite (rusqlite)</li>
                  <li>• Serde JSON</li>
                </ul>
              </div>
            </div>
          </div>

          {/* Platform */}
          <div>
            <h4 class="mb-3 font-semibold text-gray-900 dark:text-white">
              🖥️ Plataforma
            </h4>
            <p class="text-sm text-gray-700 dark:text-gray-300">
              Aplicación de escritorio nativa multiplataforma (Linux, Windows)
            </p>
          </div>

          {/* Developer Info */}
          <div class="rounded-lg bg-blue-50 p-4 dark:bg-blue-900/20">
            <h4 class="mb-2 font-semibold text-gray-900 dark:text-white">
              👨‍💻 Desarrollador
            </h4>
            <div class="space-y-1 text-sm text-gray-700 dark:text-gray-300">
              <p>
                <span class="font-medium">Nombre:</span> Alejandro Palestrini
              </p>
              <p class="flex items-center gap-2">
                <span class="font-medium">Email:</span>
                <a
                  href="mailto:apalestrini@gmail.com"
                  class="text-blue-600 hover:underline dark:text-blue-400"
                >
                  apalestrini@gmail.com
                </a>
              </p>
            </div>
          </div>

          {/* Footer */}
          <div class="border-t border-gray-200 pt-4 text-center text-xs text-gray-500 dark:border-gray-700 dark:text-gray-400">
            <p>Desarrollado con ❤️ usando tecnologías open source</p>
            <p class="mt-1">© 2025 Alejandro Palestrini</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default About;
