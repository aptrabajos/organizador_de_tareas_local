import type { ProjectAttachment } from '../types/project';

/**
 * Compone el data URL de un adjunto para poder pintarlo en un `<img src>`.
 *
 * IMPORTANTE: `file_data` se guarda como base64 CRUDO, SIN el prefijo
 * `data:<mime>;base64,`. El prefijo lo recorta `fileToBase64` al subir el
 * archivo, y `downloadAttachment` depende de esa misma forma porque hace
 * `atob(file_data)` directo. O sea: la base de datos guarda base64 pelado y
 * cada lugar que quiera MOSTRARLO tiene que recomponer el prefijo.
 *
 * Esta función vive acá, compartida, justamente para que nadie tenga que
 * acordarse de eso: `ProjectContext` hacía `src={attachment.file_data}` y la
 * miniatura no renderizaba nunca (un base64 pelado no es una URL válida).
 */
export function getImageDataUrl(
  attachment: Pick<ProjectAttachment, 'mime_type' | 'file_data'>
): string {
  return `data:${attachment.mime_type};base64,${attachment.file_data}`;
}
