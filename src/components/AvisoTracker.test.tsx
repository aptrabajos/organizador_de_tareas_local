import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import { invoke } from '@tauri-apps/api/core';
import AvisoTracker, {
  TEXTO_ALARMA,
  TEXTO_LINEA_TRABAJAR,
} from './AvisoTracker';

// Aviso de cabecera sin tracker (U6, 3.10.14). La regla (`aviso_cabecera`) vive en el
// backend y tiene su tabla en Rust; acá se prueba el cableado: qué cuenta de sondeos
// se le pasa, qué se muestra y cuándo se relee.

type InvokeImpl = (cmd: string, args?: Record<string, unknown>) => unknown;
let base: InvokeImpl;

interface Sondeo {
  alarma: boolean;
  linea_trabajar: boolean;
}

/**
 * `estado_trabajar` falso que devuelve los avisos de `respuestas` en orden (el
 * último se repite) y cuenta los sondeos sin demonio como el backend: el previo + 1.
 */
const sondeos = (respuestas: Sondeo[]) => {
  let vuelta = 0;
  vi.mocked(invoke).mockImplementation(((
    cmd: string,
    args?: Record<string, unknown>
  ) => {
    if (cmd !== 'estado_trabajar') return base(cmd, args);
    const aviso = respuestas[Math.min(vuelta, respuestas.length - 1)];
    vuelta += 1;
    const previos = (args?.sondeosSinDemonio as number | null) ?? 0;
    return Promise.resolve({
      sesiones: [],
      demonio_vivo: false,
      adopcion_imposible: false,
      tracker_corrio: true,
      sondeos_sin_demonio: previos + 1,
      aviso,
    });
  }) as typeof invoke);
};

const NADA = { alarma: false, linea_trabajar: false };
const ALARMA = { alarma: true, linea_trabajar: false };

const llamadas = () =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([cmd]) => cmd === 'estado_trabajar')
    .map(([, args]) => args);

const sondearOtraVez = () => window.dispatchEvent(new Event('focus'));

/** Deja que termine el sondeo en curso (la respuesta guarda la cuenta). */
const terminarSondeo = () => new Promise((listo) => setTimeout(listo, 0));

beforeEach(() => {
  base = vi.mocked(invoke).getMockImplementation() as InvokeImpl;
  vi.mocked(invoke).mockClear();
});

afterEach(() => {
  vi.mocked(invoke).mockImplementation(base as typeof invoke);
});

describe('AvisoTracker', () => {
  it('en el primer sondeo no hay nada que mostrar', async () => {
    sondeos([NADA]);
    render(() => <AvisoTracker />);

    await waitFor(() => {
      expect(llamadas()).toEqual([{ sondeosSinDemonio: 0 }]);
    });
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('cada sondeo le pasa al backend la cuenta que devolvió el anterior, y al segundo aparece la alarma', async () => {
    sondeos([NADA, ALARMA]);
    render(() => <AvisoTracker />);
    await waitFor(() => {
      expect(llamadas()).toHaveLength(1);
    });
    await terminarSondeo();
    expect(screen.queryByRole('alert')).toBeNull();

    // Relectura al ganar el foco: el segundo sondeo seguido sin demonio.
    sondearOtraVez();

    expect((await screen.findByRole('alert')).textContent).toContain(
      TEXTO_ALARMA
    );
    expect(llamadas()).toEqual([
      { sondeosSinDemonio: 0 },
      { sondeosSinDemonio: 1 },
    ]);
  });

  it('la alarma desaparece en el sondeo en que el tracker vuelve', async () => {
    sondeos([ALARMA, NADA]);
    render(() => <AvisoTracker />);
    await screen.findByRole('alert');

    sondearOtraVez();

    await waitFor(() => {
      expect(screen.queryByRole('alert')).toBeNull();
    });
  });

  it('con una sesión de la app muestra la línea "Trabajar mide sin vigilancia"', async () => {
    sondeos([{ alarma: false, linea_trabajar: true }]);
    render(() => <AvisoTracker />);

    expect((await screen.findByRole('status')).textContent).toContain(
      TEXTO_LINEA_TRABAJAR
    );
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('con la alarma y una sesión de la app se ven las dos', async () => {
    sondeos([{ alarma: true, linea_trabajar: true }]);
    render(() => <AvisoTracker />);

    await screen.findByRole('alert');
    expect(screen.getByRole('status')).toBeTruthy();
  });

  it('vuelve a sondear cada 5 s', async () => {
    vi.useFakeTimers();
    try {
      sondeos([NADA]);
      render(() => <AvisoTracker />);
      await vi.advanceTimersByTimeAsync(0);
      expect(llamadas()).toHaveLength(1);

      await vi.advanceTimersByTimeAsync(5000);
      expect(llamadas()).toHaveLength(2);
    } finally {
      vi.useRealTimers();
    }
  });
});
