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
  // Analytics fields
  last_opened_at?: string;
  opened_count?: number;
  total_time_seconds?: number;
}

export interface ProjectActivity {
  id: number;
  project_id: number;
  activity_type: string;
  description?: string;
  duration_seconds?: number;
  created_at: string;
}

export interface ProjectStats {
  total_projects: number;
  active_today: number;
  total_time_hours: number;
  most_active_project?: string;
  recent_activities: ProjectActivity[];
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
