import { Component, Show, createSignal, onCleanup, onMount } from 'solid-js';
import { estadoTrabajar } from '../services/api';
import type { AvisoCabecera } from '../services/api';
import { logger } from '../utils/logger';

/** Cada cuánto se sondea el estado del tracker (dos sondeos seguidos: ≥ 10 s). */
export const SONDEO_MS = 5000;

export const TEXTO_ALARMA =
  'El tracker no está corriendo: no se mide el tiempo automático; Trabajar mide sin pausa (hasta 12 h)';

export const TEXTO_LINEA_TRABAJAR =
  'Trabajar mide sin vigilancia: no se pausa por inactividad y se corta a las 12 h';

/**
 * Aviso de cabecera sin tracker (U6, 3.10.14 del diseño del tracker). La regla vive
 * en el backend (`aviso_cabecera`): acá solo se sondea `estado_trabajar` cada 5 s y
 * al ganar el foco, devolviéndole los sondeos seguidos sin demonio que contó el
 * sondeo anterior, para que la alarma necesite dos seguidos y no parpadee en un
 * reinicio del tracker.
 */
const AvisoTracker: Component = () => {
  const [aviso, setAviso] = createSignal<AvisoCabecera>({
    alarma: false,
    linea_trabajar: false,
  });
  let sondeosSinDemonio = 0;

  const sondear = async () => {
    try {
      const estado = await estadoTrabajar(sondeosSinDemonio);
      sondeosSinDemonio = estado.sondeos_sin_demonio;
      setAviso(estado.aviso);
    } catch (error) {
      logger.error('Error leyendo el estado del tracker:', error);
    }
  };

  onMount(() => {
    void sondear();
    const intervalo = window.setInterval(() => void sondear(), SONDEO_MS);
    window.addEventListener('focus', sondear);
    onCleanup(() => {
      window.clearInterval(intervalo);
      window.removeEventListener('focus', sondear);
    });
  });

  return (
    <>
      <Show when={aviso().alarma}>
        <p
          role="alert"
          class="mt-3 rounded-lg border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-800 dark:border-red-800 dark:bg-red-950 dark:text-red-200"
        >
          ⚠️ {TEXTO_ALARMA}
        </p>
      </Show>
      <Show when={aviso().linea_trabajar}>
        <p
          role="status"
          class="mt-3 rounded-lg border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-800 dark:border-amber-800 dark:bg-amber-950 dark:text-amber-200"
        >
          {TEXTO_LINEA_TRABAJAR}
        </p>
      </Show>
    </>
  );
};

export default AvisoTracker;
