import { describe, it, expect } from 'vitest';
import { shouldConfirm } from './confirm';
import type { AppConfig } from '../types/config';

const withConfirmDelete = (value: boolean): Pick<AppConfig, 'ui'> => ({
  ui: {
    theme: 'auto',
    language: 'es',
    confirm_delete: value,
    show_welcome: true,
  },
});

describe('shouldConfirm', () => {
  describe('operaciones reversibles', () => {
    it('confirma cuando el flag está encendido', () => {
      expect(shouldConfirm('reversible', withConfirmDelete(true))).toBe(true);
    });

    it('NO confirma cuando el usuario apagó el flag', () => {
      // Este es el bug original: la preferencia se guardaba y no la leía nadie,
      // así que la app seguía preguntando igual.
      expect(shouldConfirm('reversible', withConfirmDelete(false))).toBe(false);
    });
  });

  describe('operaciones irreversibles', () => {
    it('confirma IGUAL aunque el flag esté apagado', () => {
      // El caso que importa: apagar una preferencia de comodidad no puede
      // habilitar el borrado definitivo con un solo click.
      expect(shouldConfirm('irreversible', withConfirmDelete(false))).toBe(true);
    });

    it('confirma con el flag encendido', () => {
      expect(shouldConfirm('irreversible', withConfirmDelete(true))).toBe(true);
    });

    it('confirma sin config', () => {
      expect(shouldConfirm('irreversible', null)).toBe(true);
    });
  });

  describe('sin config cargada', () => {
    it('cae en confirmar ante null', () => {
      // Ante la duda, en un camino destructivo se pregunta.
      expect(shouldConfirm('reversible', null)).toBe(true);
    });

    it('cae en confirmar ante undefined', () => {
      expect(shouldConfirm('reversible', undefined)).toBe(true);
    });
  });
});
