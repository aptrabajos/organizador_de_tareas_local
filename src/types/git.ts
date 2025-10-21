// ==================== TIPOS GIT ====================

// Información de un commit
export interface GitCommit {
  hash: string;
  author: string;
  date: string;
  message: string;
}

// Conteo de archivos modificados
export interface GitFileCount {
  modified: number;
  staged: number;
  untracked: number;
}

// Información del remote
export interface GitRemoteInfo {
  url: string | null;
  ahead: number;
  behind: number;
}

// Estado completo de Git para un proyecto
export interface GitStatus {
  branch: string;
  hasChanges: boolean;
  fileCount: GitFileCount;
  remoteInfo: GitRemoteInfo;
  recentCommits: GitCommit[];
}
