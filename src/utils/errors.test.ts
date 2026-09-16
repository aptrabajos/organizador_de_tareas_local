import { describe, it, expect } from 'vitest';
import { getErrorMessage } from './errors';

describe('getErrorMessage', () => {
  // Caso Tauri v2: un comando Rust que devuelve `Err(String)` rechaza la promesa
  // con un STRING PLANO. Es el caso que el patrón `err instanceof Error` tiraba
  // a la basura, y justo el que trae el mensaje accionable para el usuario.
  it('devuelve el string tal cual cuando el error es un string plano', () => {
    expect(
      getErrorMessage('No podés asignar un proyecto como su propio grupo padre.')
    ).toBe('No podés asignar un proyecto como su propio grupo padre.');
  });

  it('devuelve el string vacío sin convertirlo en "Error desconocido"', () => {
    // Un string vacío sigue siendo un string: no hay que disfrazarlo de otra cosa.
    expect(getErrorMessage('')).toBe('');
  });

  it('devuelve el message cuando el error es una instancia de Error', () => {
    expect(getErrorMessage(new Error('Falló la conexión'))).toBe(
      'Falló la conexión'
    );
  });

  it('devuelve el message de subclases de Error', () => {
    expect(getErrorMessage(new TypeError('no es una función'))).toBe(
      'no es una función'
    );
  });

  it('cae en "Error desconocido" cuando el error es un objeto arbitrario', () => {
    expect(getErrorMessage({ code: 500, detail: 'boom' })).toBe(
      'Error desconocido'
    );
  });

  it('cae en "Error desconocido" con null, undefined y números', () => {
    expect(getErrorMessage(null)).toBe('Error desconocido');
    expect(getErrorMessage(undefined)).toBe('Error desconocido');
    expect(getErrorMessage(42)).toBe('Error desconocido');
  });
});
