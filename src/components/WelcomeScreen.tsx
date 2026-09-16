import { createSignal, For, Index } from 'solid-js';
import { getConfig, updateConfig } from '../services/api';
import { logger } from '../utils/logger';

export default function WelcomeScreen(props: { onClose: () => void }) {
  const [currentStep, setCurrentStep] = createSignal(0);

  const steps = [
    {
      title: '¡Bienvenido a Gestor de Proyectos!',
      icon: '👋',
      description:
        'Una aplicación de escritorio para gestionar tus proyectos de manera visual y eficiente.',
      features: [
        '📁 Gestión completa de proyectos locales',
        '🔗 Enlaces y recursos organizados',
        '📊 Tracking de actividad y progreso',
        '📓 Diario y TODOs por proyecto',
        '⚙️ Configuración multiplataforma',
      ],
    },
    {
      title: 'Características Principales',
      icon: '✨',
      description: 'Todo lo que necesitas en una sola aplicación:',
      features: [
        '🚀 Abrir terminal en el proyecto con un click',
        '📝 Editor Markdown con preview en tiempo real',
        '📎 Adjuntar archivos importantes',
        '🎨 Dark mode y temas personalizables',
        '🔍 Búsqueda y filtros avanzados',
        '⭐ Sistema de favoritos',
      ],
    },
    {
      title: 'Primeros Pasos',
      icon: '🎯',
      description: 'Para empezar a usar la aplicación:',
      features: [
        '1️⃣ Crea tu primer proyecto con el botón "+ Nuevo Proyecto"',
        '2️⃣ Configura tus programas favoritos en "⚙️ Configuración"',
        '3️⃣ Usa "🚀 Trabajar" para abrir el terminal del proyecto',
        '4️⃣ Agrega enlaces, notas y TODOs según necesites',
        '5️⃣ Organiza tus proyectos en grupos y usa drag & drop',
      ],
    },
  ];

  const currentStepData = () => steps[currentStep()];
  const isLastStep = () => currentStep() === steps.length - 1;

  const handleNext = () => {
    if (isLastStep()) {
      handleClose();
    } else {
      setCurrentStep((prev) => prev + 1);
    }
  };

  const handlePrevious = () => {
    setCurrentStep((prev) => Math.max(0, prev - 1));
  };

  const handleClose = async () => {
    // Actualizar config para no mostrar bienvenida de nuevo
    try {
      const config = await getConfig();
      await updateConfig({
        ...config,
        ui: {
          ...config.ui,
          show_welcome: false,
        },
      });
    } catch (err) {
      logger.error('Error actualizando config:', err);
    }
    props.onClose();
  };

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 p-4">
      <div class="flex max-h-[90vh] w-full max-w-3xl flex-col overflow-hidden rounded-lg bg-white shadow-2xl dark:bg-gray-900">
        {/* Header */}
        <div class="flex items-center justify-between border-b border-gray-200 p-6 dark:border-gray-700">
          <div class="flex items-center gap-3">
            <span class="text-4xl">{currentStepData().icon}</span>
            <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
              {currentStepData().title}
            </h2>
          </div>
          <button
            onClick={handleClose}
            class="text-2xl text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
          >
            ×
          </button>
        </div>

        {/* Content */}
        <div class="flex-1 overflow-y-auto p-8">
          <p class="mb-6 text-lg text-gray-600 dark:text-gray-400">
            {currentStepData().description}
          </p>

          <div class="space-y-4">
            <For each={currentStepData().features}>
              {(feature) => (
                <div class="flex items-start gap-3 rounded-lg bg-gray-50 p-4 dark:bg-gray-800">
                  <span class="text-2xl">{feature.split(' ')[0]}</span>
                  <p class="flex-1 text-gray-800 dark:text-gray-200">
                    {feature.substring(feature.indexOf(' ') + 1)}
                  </p>
                </div>
              )}
            </For>
          </div>

          {/* Step indicator */}
          <div class="mt-8 flex justify-center gap-2">
            <Index each={steps}>
              {(_step, index) => (
                <button
                  onClick={() => setCurrentStep(index)}
                  class={`h-2 w-2 rounded-full transition-all ${
                    index === currentStep()
                      ? 'w-8 bg-blue-600'
                      : 'bg-gray-300 hover:bg-gray-400 dark:bg-gray-600 dark:hover:bg-gray-500'
                  }`}
                />
              )}
            </Index>
          </div>
        </div>

        {/* Footer */}
        <div class="flex items-center justify-between border-t border-gray-200 bg-gray-50 p-6 dark:border-gray-700 dark:bg-gray-800">
          <button
            onClick={handlePrevious}
            disabled={currentStep() === 0}
            class="rounded-md px-6 py-2 text-gray-700 transition-colors hover:bg-gray-200 disabled:cursor-not-allowed disabled:opacity-50 dark:text-gray-300 dark:hover:bg-gray-700"
          >
            ← Anterior
          </button>

          <button
            onClick={handleNext}
            class="rounded-md bg-blue-500 px-6 py-2 text-white transition-colors hover:bg-blue-600"
          >
            {isLastStep() ? '¡Empezar! 🚀' : 'Siguiente →'}
          </button>
        </div>
      </div>
    </div>
  );
}
