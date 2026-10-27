import { describe, it, expect } from 'vitest';
import { fechaDeLaBase } from './fechas';

describe('fechaDeLaBase', () => {
  it('lee el formato de SQLite como UTC', () => {
    expect(fechaDeLaBase('2026-10-27 00:23:44').toISOString()).toBe(
      '2026-10-27T00:23:44.000Z'
    );
  });

  it('acepta la T y los milisegundos sin zona como UTC', () => {
    expect(fechaDeLaBase('2026-10-27T00:23:44.5').toISOString()).toBe(
      '2026-10-27T00:23:44.500Z'
    );
  });

  it('respeta una fecha que ya trae zona', () => {
    expect(fechaDeLaBase('2024-01-01T00:00:00Z').toISOString()).toBe(
      '2024-01-01T00:00:00.000Z'
    );
    expect(fechaDeLaBase('2026-10-26T21:23:44-03:00').toISOString()).toBe(
      '2026-10-27T00:23:44.000Z'
    );
  });

  it('un texto inválido da una fecha inválida, no una excepción', () => {
    expect(Number.isNaN(fechaDeLaBase('no es fecha').getTime())).toBe(true);
  });
});
