import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { logger } from './logger';

/**
 * El valor entero del helper es la guarda de entorno. Si la guarda se rompe —o
 * si alguien "simplifica" capturando `import.meta.env.DEV` en una const de
 * módulo— el diagnóstico vuelve a filtrarse al build del usuario y nada más
 * falla. Estos tests son lo único que lo detecta.
 */
describe('logger', () => {
  let spies: ReturnType<typeof vi.spyOn>[];

  beforeEach(() => {
    spies = [
      vi.spyOn(console, 'log').mockImplementation(() => {}),
      vi.spyOn(console, 'warn').mockImplementation(() => {}),
      vi.spyOn(console, 'error').mockImplementation(() => {}),
    ];
  });

  afterEach(() => {
    vi.unstubAllEnvs();
    spies.forEach((spy) => spy.mockRestore());
  });

  it('en desarrollo escribe en la consola con el método correspondiente', () => {
    vi.stubEnv('DEV', true);

    logger.debug('hola', 1);
    logger.warn('ojo');
    logger.error('rompió');

    expect(console.log).toHaveBeenCalledWith('hola', 1);
    expect(console.warn).toHaveBeenCalledWith('ojo');
    expect(console.error).toHaveBeenCalledWith('rompió');
  });

  it('en producción NO emite nada, ni siquiera los errores', () => {
    vi.stubEnv('DEV', false);

    logger.debug('datos del proyecto', { id: 1 });
    logger.warn('algo raro');
    logger.error('rompió', new Error('boom'));

    expect(console.log).not.toHaveBeenCalled();
    expect(console.warn).not.toHaveBeenCalled();
    // `error` incluido a propósito: con las devtools apagadas en el build de
    // producción nadie puede leer esa consola. Ver el comentario del módulo.
    expect(console.error).not.toHaveBeenCalled();
  });

  it('no evalúa los argumentos de más en producción (pasa la lista tal cual en dev)', () => {
    vi.stubEnv('DEV', true);
    logger.debug('a', 'b', 'c');
    expect(console.log).toHaveBeenCalledWith('a', 'b', 'c');
  });
});
