// ==================== TIPOS GIT ====================

// Información de un commit.
//
// Única definición del tipo. Estaba duplicado en types/project.ts y cada
// consumidor importaba uno distinto: eran estructuralmente compatibles, así
// que TypeScript no se quejaba, pero el día que uno cambiara el error iba a
// aparecer lejos del cambio. Git es el dominio de este archivo, así que la
// definición vive acá.
export interface GitCommit {
  hash: string;
  author: string;
  date: string;
  message: string;
}

// Conteo de archivos modificados. Lo devuelve `get_git_file_count`.
export interface GitFileCount {
  modified: number;
  staged: number;
  untracked: number;
}
