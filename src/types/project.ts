export interface ProjectLink {
  id: number;
  project_id: number;
  type:
    | 'repository'
    | 'documentation'
    | 'staging'
    | 'production'
    | 'design'
    | 'api'
    | 'other';
  title: string;
  url: string;
  created_at: string;
}

export interface Project {
  id: number;
  name: string;
  description: string;
  local_path: string;
  documentation_url?: string;
  ai_documentation_url?: string;
  drive_link?: string;
  notes?: string;
  image_data?: string;
  links?: ProjectLink[];
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
  notes?: string;
  image_data?: string;
}

export interface UpdateProjectDTO {
  name?: string;
  description?: string;
  local_path?: string;
  documentation_url?: string;
  ai_documentation_url?: string;
  drive_link?: string;
  notes?: string;
  image_data?: string;
}
