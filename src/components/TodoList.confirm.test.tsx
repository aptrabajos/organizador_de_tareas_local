import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import userEvent from '@testing-library/user-event';
import TodoList from './TodoList';
import { ConfigProvider } from '../contexts/ConfigContext';
import { setUiConfigOverrides } from '../test-setup';
import type { ProjectTodo } from '../types/project';

/**
 * Guardia del CABLEADO de `ui.confirm_delete`, no de la lógica.
 *
 * `src/utils/confirm.test.ts` ya prueba que `shouldConfirm` decide bien. Esto es
 * otra cosa: prueba que el componente REALMENTE la llama. Sin esta red, alguien
 * puede sacar el `shouldConfirm(...)` de un handler en un refactor, `confirm.test.ts`
 * sigue en verde, y el bug vuelve intacto — que es exactamente cómo sobrevivió B7.
 *
 * Alcance a propósito: acá se protege el PATRÓN, no cada instancia. `ProjectJournal`,
 * `ProjectLinks` y `AttachmentManager` usan la misma forma
 * (`shouldConfirm('reversible', configCtx.config()) && !confirm(...)`) con el mismo
 * `window.confirm` sincrónico; duplicar el test cuatro veces agrega mantenimiento
 * sin agregar señal. El camino irreversible, que es el destructivo, tiene su propia
 * guardia en `TrashModal.confirm.test.tsx`.
 */

vi.mock('../services/api', async () => {
  const actual =
    await vi.importActual<typeof import('../services/api')>('../services/api');
  return {
    ...actual,
    getProjectTodos: vi.fn(),
    deleteTodo: vi.fn(),
  };
});

import { getProjectTodos, deleteTodo } from '../services/api';

const mockTodos: ProjectTodo[] = [
  {
    id: 1,
    project_id: 1,
    content: 'Tarea de prueba',
    is_completed: false,
    created_at: '2025-01-01T00:00:00Z',
  },
];

const renderTodoList = () =>
  render(() => (
    <ConfigProvider>
      <TodoList projectId={1} />
    </ConfigProvider>
  ));

const clickDelete = async () => {
  const user = userEvent.setup();
  const buttons = screen.getAllByRole('button', { name: /eliminar tarea/i });
  await user.click(buttons[0]);
};

describe('TodoList — cableado de ui.confirm_delete (borrado reversible)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getProjectTodos).mockResolvedValue(mockTodos);
    vi.mocked(deleteTodo).mockResolvedValue(undefined);
  });

  it('con confirm_delete=false NO pide confirmación y borra igual', async () => {
    setUiConfigOverrides({ confirm_delete: false });
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);

    renderTodoList();
    await screen.findByText('Tarea de prueba');
    // Esperamos a que la config resuelva: antes de eso el default es confirmar.
    await waitFor(() => expect(confirmSpy).not.toHaveBeenCalled());

    await clickDelete();

    await waitFor(() => expect(deleteTodo).toHaveBeenCalledWith(1));
    expect(confirmSpy).not.toHaveBeenCalled();

    confirmSpy.mockRestore();
  });

  it('con confirm_delete=true SÍ pide confirmación', async () => {
    setUiConfigOverrides({ confirm_delete: true });
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);

    renderTodoList();
    await screen.findByText('Tarea de prueba');

    await clickDelete();

    await waitFor(() => expect(confirmSpy).toHaveBeenCalledTimes(1));
    expect(deleteTodo).toHaveBeenCalledWith(1);

    confirmSpy.mockRestore();
  });

  it('con confirm_delete=true y el usuario cancelando, NO borra', async () => {
    setUiConfigOverrides({ confirm_delete: true });
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);

    renderTodoList();
    await screen.findByText('Tarea de prueba');

    await clickDelete();

    await waitFor(() => expect(confirmSpy).toHaveBeenCalledTimes(1));
    expect(deleteTodo).not.toHaveBeenCalled();

    confirmSpy.mockRestore();
  });
});
