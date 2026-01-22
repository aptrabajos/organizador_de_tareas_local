// Manejo de la carpeta .gestor/ en cada proyecto
// Similar a .git/ pero para tracking de tiempo

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Nombre del directorio de configuración del gestor
pub const GESTOR_DIR_NAME: &str = ".gestor";
/// Nombre del archivo de configuración dentro del directorio
pub const CONFIG_FILE_NAME: &str = "config.json";

/// Configuración de tracking para un proyecto específico
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GestorConfig {
    /// ID del proyecto en la base de datos
    pub project_id: i64,
    /// Nombre del proyecto (para verificación)
    pub project_name: String,
    /// Fecha de creación de la configuración
    pub created_at: String,
    /// Si el tracking está habilitado
    pub tracking_enabled: bool,
}

impl GestorConfig {
    /// Crear una nueva configuración de tracking
    pub fn new(project_id: i64, project_name: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            project_id,
            project_name,
            created_at: now,
            tracking_enabled: true,
        }
    }

    /// Guardar la configuración en el directorio especificado
    pub fn save(&self, project_path: &Path) -> Result<PathBuf, ConfigError> {
        let gestor_dir = project_path.join(GESTOR_DIR_NAME);

        // Crear directorio .gestor/ si no existe
        fs::create_dir_all(&gestor_dir)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        let config_path = gestor_dir.join(CONFIG_FILE_NAME);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializationError(e.to_string()))?;

        fs::write(&config_path, json)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        Ok(config_path)
    }

    /// Cargar configuración desde un directorio de proyecto
    pub fn load(project_path: &Path) -> Result<Self, ConfigError> {
        let config_path = project_path.join(GESTOR_DIR_NAME).join(CONFIG_FILE_NAME);

        if !config_path.exists() {
            return Err(ConfigError::NotFound(config_path.to_string_lossy().to_string()));
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        serde_json::from_str(&content)
            .map_err(|e| ConfigError::DeserializationError(e.to_string()))
    }
}

/// Errores posibles al manejar la configuración
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    NotFound(String),
    IoError(String),
    SerializationError(String),
    DeserializationError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NotFound(path) => write!(f, "Config not found: {}", path),
            ConfigError::IoError(msg) => write!(f, "IO error: {}", msg),
            ConfigError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            ConfigError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Inicializar la carpeta .gestor/ en un proyecto
pub fn init_gestor_config(project_path: &Path, project_id: i64, project_name: String) -> Result<GestorConfig, ConfigError> {
    let config = GestorConfig::new(project_id, project_name);
    config.save(project_path)?;
    Ok(config)
}

/// Buscar .gestor/config.json hacia arriba en la jerarquía de directorios
/// Retorna el path del directorio del proyecto (no del .gestor/)
pub fn find_gestor_config(start_path: &Path) -> Option<PathBuf> {
    let mut current = start_path.to_path_buf();

    loop {
        let gestor_path = current.join(GESTOR_DIR_NAME).join(CONFIG_FILE_NAME);
        if gestor_path.exists() {
            return Some(current);
        }

        // Subir al directorio padre
        if !current.pop() {
            break;
        }
    }

    None
}

/// Verificar si un directorio tiene configuración de tracking
pub fn has_tracking_config(path: &Path) -> bool {
    let config_path = path.join(GESTOR_DIR_NAME).join(CONFIG_FILE_NAME);
    config_path.exists()
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_gestor_config_serialization() {
        let config = GestorConfig::new(123, "Test Project".to_string());

        // Serializar
        let json = serde_json::to_string(&config).expect("Should serialize");

        // Deserializar
        let parsed: GestorConfig = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(parsed.project_id, 123);
        assert_eq!(parsed.project_name, "Test Project");
        assert!(parsed.tracking_enabled);
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let project_path = temp_dir.path();

        // Crear y guardar config
        let config = GestorConfig::new(456, "My Project".to_string());
        let saved_path = config.save(project_path).expect("Should save config");

        // Verificar que el archivo existe
        assert!(saved_path.exists());
        assert!(saved_path.to_string_lossy().contains(".gestor"));
        assert!(saved_path.to_string_lossy().contains("config.json"));

        // Cargar config
        let loaded = GestorConfig::load(project_path).expect("Should load config");

        assert_eq!(loaded.project_id, 456);
        assert_eq!(loaded.project_name, "My Project");
        assert!(loaded.tracking_enabled);
    }

    #[test]
    fn test_find_config_in_current_dir() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let project_path = temp_dir.path();

        // Crear config en el directorio actual
        init_gestor_config(project_path, 789, "Current Project".to_string())
            .expect("Should init config");

        // Buscar config desde el mismo directorio
        let found = find_gestor_config(project_path);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), project_path);
    }

    #[test]
    fn test_find_config_in_ancestors() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let project_path = temp_dir.path();

        // Crear estructura: project/src/components/
        let nested_path = project_path.join("src").join("components");
        fs::create_dir_all(&nested_path).expect("Should create nested dirs");

        // Inicializar config en el raíz del proyecto
        init_gestor_config(project_path, 999, "Nested Project".to_string())
            .expect("Should init config");

        // Buscar config desde subdirectorio anidado
        let found = find_gestor_config(&nested_path);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), project_path);
    }

    #[test]
    fn test_find_config_not_found() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let empty_path = temp_dir.path();

        // No hay config, debería retornar None
        let found = find_gestor_config(empty_path);
        assert!(found.is_none());
    }

    #[test]
    fn test_load_nonexistent_config() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let empty_path = temp_dir.path();

        // Intentar cargar config que no existe
        let result = GestorConfig::load(empty_path);
        assert!(result.is_err());

        match result {
            Err(ConfigError::NotFound(_)) => (), // Esperado
            _ => panic!("Should return NotFound error"),
        }
    }

    #[test]
    fn test_has_tracking_config() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let project_path = temp_dir.path();

        // Sin config
        assert!(!has_tracking_config(project_path));

        // Crear config
        init_gestor_config(project_path, 111, "Test".to_string())
            .expect("Should init");

        // Con config
        assert!(has_tracking_config(project_path));
    }

    #[test]
    fn test_config_json_structure() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let project_path = temp_dir.path();

        // Crear config
        let config = GestorConfig::new(222, "JSON Test".to_string());
        config.save(project_path).expect("Should save");

        // Leer JSON directamente
        let config_path = project_path.join(GESTOR_DIR_NAME).join(CONFIG_FILE_NAME);
        let content = fs::read_to_string(config_path).expect("Should read file");

        // Verificar estructura JSON
        assert!(content.contains("\"project_id\": 222"));
        assert!(content.contains("\"project_name\": \"JSON Test\""));
        assert!(content.contains("\"tracking_enabled\": true"));
        assert!(content.contains("\"created_at\""));
    }
}
