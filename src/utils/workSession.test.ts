import { describe, it, expect, vi, beforeEach } from 'vitest';
import toast from 'solid-toast';
import { avisarSesionesCortadas, formatStoppedDuration } from './workSession';

beforeEach(() => {
  vi.mocked(toast).mockClear();
});

describe('avisarSesionesCortadas (papelera, 3.10.9)', () => {
  it('avisa cada sesión cortada con su proyecto y su duración', () => {
    avisarSesionesCortadas([
      {
        token: '0123456789abcdef0123456789abcdef',
        project_id: 1,
        nombre: 'Padre',
        segundos: 4320,
      },
      {
        token: 'fedcba9876543210fedcba9876543210',
        project_id: 2,
        nombre: 'Hijo',
        segundos: 417,
      },
    ]);

    expect(toast).toHaveBeenCalledTimes(2);
    expect(toast).toHaveBeenCalledWith(
      'Se cortó la sesión de Padre (1h 12m) porque lo mandaste a la papelera.',
      expect.anything()
    );
    expect(toast).toHaveBeenCalledWith(
      'Se cortó la sesión de Hijo (6m 57s) porque lo mandaste a la papelera.',
      expect.anything()
    );
  });

  it('sin sesiones cortadas no avisa nada', () => {
    avisarSesionesCortadas([]);
    expect(toast).not.toHaveBeenCalled();
  });
});

describe('formatStoppedDuration', () => {
  it('muestra horas y minutos, minutos y segundos, o segundos', () => {
    expect(formatStoppedDuration(4320)).toBe('1h 12m');
    expect(formatStoppedDuration(3600)).toBe('1h');
    expect(formatStoppedDuration(417)).toBe('6m 57s');
    expect(formatStoppedDuration(180)).toBe('3m');
    expect(formatStoppedDuration(40)).toBe('40 s');
  });
});
