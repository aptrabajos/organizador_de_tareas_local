import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@solidjs/testing-library';
import userEvent from '@testing-library/user-event';
import SearchBar from './SearchBar';

describe('SearchBar', () => {
  it('should render input with placeholder', () => {
    render(() => <SearchBar onSearch={vi.fn()} />);

    const input = screen.getByPlaceholderText(/buscar proyecto/i);
    expect(input).toBeTruthy();
  });

  it('should call onSearch when typing', async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn();

    render(() => <SearchBar onSearch={onSearch} />);

    const input = screen.getByPlaceholderText(/buscar proyecto/i);
    await user.type(input, 'test');

    expect(onSearch).toHaveBeenCalled();
    expect(onSearch).toHaveBeenLastCalledWith('test');
  });

  it('should display current value', () => {
    render(() => <SearchBar onSearch={vi.fn()} value="initial" />);

    const input = screen.getByPlaceholderText(
      /buscar proyecto/i
    ) as HTMLInputElement;
    expect(input.value).toBe('initial');
  });

  it('should clear search when clear button is clicked', async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn();

    render(() => <SearchBar onSearch={onSearch} value="test" />);

    const clearButton = screen.getByRole('button', { name: /limpiar/i });
    await user.click(clearButton);

    expect(onSearch).toHaveBeenCalledWith('');
  });

  it('should not show clear button when value is empty', () => {
    render(() => <SearchBar onSearch={vi.fn()} value="" />);

    const clearButton = screen.queryByRole('button', { name: /limpiar/i });
    expect(clearButton).toBeFalsy();
  });

  it('should show clear button when value is not empty', () => {
    render(() => <SearchBar onSearch={vi.fn()} value="test" />);

    const clearButton = screen.getByRole('button', { name: /limpiar/i });
    expect(clearButton).toBeTruthy();
  });
});
