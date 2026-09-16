import { describe, it, expect } from 'vitest';
import { getImageDataUrl } from './attachments';

describe('getImageDataUrl', () => {
  it('compone el data URL con el mime type y el base64 crudo', () => {
    expect(
      getImageDataUrl({ mime_type: 'image/png', file_data: 'iVBORw0KGgo=' })
    ).toBe('data:image/png;base64,iVBORw0KGgo=');
  });

  it('respeta el mime type que le pasan', () => {
    // No hardcodea image/png: un JPEG o un SVG tienen que salir con SU mime.
    expect(
      getImageDataUrl({ mime_type: 'image/jpeg', file_data: '/9j/4AAQ' })
    ).toBe('data:image/jpeg;base64,/9j/4AAQ');

    expect(
      getImageDataUrl({ mime_type: 'image/svg+xml', file_data: 'PHN2Zz4=' })
    ).toBe('data:image/svg+xml;base64,PHN2Zz4=');
  });

  it('produce un src que arranca con el prefijo data:', () => {
    // La razón de ser del helper: un base64 pelado NO es una URL válida para
    // un `<img src>`. Este es el chequeo que atrapa el bug original.
    const src = getImageDataUrl({
      mime_type: 'image/webp',
      file_data: 'UklGRg==',
    });

    expect(src.startsWith('data:image/webp;base64,')).toBe(true);
    expect(src).not.toBe('UklGRg==');
  });

  it('no rompe con file_data vacío', () => {
    expect(getImageDataUrl({ mime_type: 'image/png', file_data: '' })).toBe(
      'data:image/png;base64,'
    );
  });
});
