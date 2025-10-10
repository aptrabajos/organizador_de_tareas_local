export interface Project {
  id: number;
  name: string;
  description: string;
  local_path: string;
  documentation_url?: string;
  ai_documentation_url?: string;
  drive_link?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateProjectDTO {
  name: string;
  description: string;
  local_path: string;
  documentation_url?: string;
  ai_documentation_url?: string;
  drive_link?: string;
}

export interface UpdateProjectDTO {
  name?: string;
  description?: string;
  local_path?: string;
  documentation_url?: string;
  ai_documentation_url?: string;
  drive_link?: string;
}
