// Módulo de Time Tracking para Gestor de Proyectos
//
// El tiempo se registra por el camino de `work_session`: el botón "Trabajar"
// llama a `start_work_session`, el backend mide con `Instant` y cierra la sesión
// al abrir otra o al cerrar la ventana.
//
// `config` maneja el archivo `.gestor/config.json` que marca un proyecto como
// trackeable. `aggregator` aporta los tipos de datos de sesión y estadísticas.
//
// El servidor de socket (`SocketServer`) y el `SessionManager` en memoria vivían
// acá, implementados y testeados pero sin arrancarse nunca en producción. Se
// eliminaron en el cierre del B17: ver ARQUITECTURA.md.

pub mod config;
pub mod aggregator;
