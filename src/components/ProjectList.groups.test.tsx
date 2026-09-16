import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import { invoke } from '@tauri-apps/api/core';
import ProjectList from './ProjectList';
import { setSubprojectCounts } from '../test-setup';
import type { Project } from '../types/project';

/**
 * Detección de grupos en ProjectList (B18).
 *
 * `ProjectList.test.tsx` tiene 8 casos y NINGUNO cubre la vista de grupos, que
 * es justo donde estaba el problema: un `for` con `await` adentro hacía N
 * llamadas ENCADENADAS, y encima cada `GroupCard` renderizada volvía a pedir en
 * su `onMount` el mismo conteo que el padre ya tenía.
 *
 * El segundo test es el que importa: cuenta las invocaciones a
 * `count_subprojects`. Sin él, alguien reintroduce el `onMount` de GroupCard y
 * nada se pone en rojo — la UI se ve idéntica, solo hace el doble de viajes.
 */

const proyecto = (id: number, name: string): Project => ({
  id,
  name,
  description: `Descripción de ${name}`,
  local_path: `/home/user/${name}`,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
});

const tresProyectos = [
  proyecto(1, 'Grupo Uno'),
  proyecto(2, 'Suelto'),
  proyecto(3, 'Grupo Dos'),
];

const renderGroups = () =>
  render(() => (
    <ProjectList
      projects={tresProyectos}
      viewMode="groups"
      onEdit={vi.fn()}
      onDelete={vi.fn()}
      onOpenTerminal={vi.fn()}
    />
  ));

/** Cuántas veces se le pidió el conteo al backend, y para qué ids. */
const countCalls = () =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([cmd]) => cmd === 'count_subprojects')
    .map(([, args]) => (args as { parentId: number }).parentId);

describe('ProjectList — detección de grupos', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockClear();
    // Dos de los tres tienen hijos.
    setSubprojectCounts({ 1: 2, 2: 0, 3: 5 });
  });

  it('renderiza como grupo solo a los proyectos que tienen hijos', async () => {
    renderGroups();

    // El badge con el conteo es exclusivo de GroupCard.
    await waitFor(() => {
      expect(screen.getByText(/2 proyectos/)).toBeTruthy();
    });
    expect(screen.getByText(/5 proyectos/)).toBeTruthy();

    // Los tres nombres están en pantalla, pero 'Suelto' no es una GroupCard.
    expect(screen.getByText('Grupo Uno')).toBeTruthy();
    expect(screen.getByText('Suelto')).toBeTruthy();
    expect(screen.getByText('Grupo Dos')).toBeTruthy();
  });

  it('pide el conteo UNA sola vez por proyecto, y nunca dos por el mismo', async () => {
    renderGroups();

    await waitFor(() => {
      expect(screen.getByText(/2 proyectos/)).toBeTruthy();
    });

    const ids = countCalls();
    // Uno por proyecto de la lista: ni uno más.
    expect(ids).toHaveLength(tresProyectos.length);
    // Y ningún id repetido: si GroupCard vuelve a pedir lo suyo, los ids 1 y 3
    // aparecen dos veces y este assert es el que se pone en rojo.
    expect(new Set(ids).size).toBe(ids.length);
    expect([...ids].sort()).toEqual([1, 2, 3]);
  });

  it('un conteo que falla no se lleva puestos a los demás grupos', async () => {
    // `Promise.all` pelado descartaría TODO el lote ante un solo rechazo. El
    // manejo es por proyecto justamente para que el resto sobreviva.
    const original = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation((cmd: string, args?: unknown) => {
      if (
        cmd === 'count_subprojects' &&
        (args as { parentId: number }).parentId === 1
      ) {
        return Promise.reject('falló el conteo del proyecto 1');
      }
      return original(cmd, args as Record<string, unknown>);
    });

    renderGroups();

    // El grupo 3 sigue detectándose aunque el conteo del 1 se haya roto.
    await waitFor(() => {
      expect(screen.getByText(/5 proyectos/)).toBeTruthy();
    });
  });
});
