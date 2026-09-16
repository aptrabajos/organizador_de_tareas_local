# 📚 Índice de documentación

Mapa completo de la documentación del repo. Una línea por documento.

**Regla de oro:** el número de versión vive en `package.json` y nada más.
`src-tauri/Cargo.toml` y `src-tauri/tauri.conf.json` lo acompañan. Ningún
documento repite la versión — se desincroniza siempre.

---

## Raíz del repo (documentación viva)

Solo cinco documentos viven en la raíz. Si agregás uno nuevo, va en `docs/`.

| Documento | Qué es |
| --------- | ------ |
| [`../README.md`](../README.md) | Puerta de entrada: qué es la app, prerequisitos, cómo levantarla en desarrollo, cómo buildear. |
| [`../ARQUITECTURA.md`](../ARQUITECTURA.md) | Arquitectura completa: por qué Vite, flujo dev/prod, puente frontend↔backend, y los módulos del backend Rust (`config/`, `platform/`, `pdf_export/`, `tracking/`). |
| [`../CLAUDE.md`](../CLAUDE.md) | Guía para trabajar el repo con Claude Code: stack, comandos, convenciones. |
| [`../CHANGELOG.md`](../CHANGELOG.md) | Registro de cambios por versión. |
| [`../TROUBLESHOOTING.md`](../TROUBLESHOOTING.md) | Problemas conocidos y sus soluciones (puerto 1420 ocupado, logs de Rust, etc.). |

## `docs/` — documentación vigente

| Documento | Qué es |
| --------- | ------ |
| [`GUIA-DESARROLLO.md`](./GUIA-DESARROLLO.md) | Guía detallada de desarrollo: cómo levantar la app nativa, capturar logs, matar instancias colgadas. |
| [`ESTADO-FUNCIONALIDADES.md`](./ESTADO-FUNCIONALIDADES.md) | Inventario de funcionalidades implementadas y pendientes. Última actualización: octubre 2025. |
| [`cli.md`](./cli.md) | La CLI `gestor`: menú `fzf` y comandos directos para saltar a la carpeta de cualquier proyecto desde la terminal. Herramienta **externa** a la app; explica por qué el `cd` necesita una función de shell y no puede vivir en un script. |
| [`reporte-fixes-v0.6.1.md`](./reporte-fixes-v0.6.1.md) | Reporte de cierre del plan de arreglo de bugs: 26 de 27 bugs corregidos, las dos decisiones de producto que quedaron (B17, B14), cómo se validó y qué falta. **Histórico de una entrega**, por eso lleva la versión en el nombre. |

## `docs/sesiones/` — bitácoras de sesiones de trabajo

Registros puntuales de sesiones de desarrollo. Son **históricos**: describen lo
que pasó ese día, no el estado actual del proyecto.

| Documento | Qué es |
| --------- | ------ |
| [`sesiones/CHANGELOG-SESION.md`](./sesiones/CHANGELOG-SESION.md) | Sesión 2025-10-10: análisis, puesta en marcha y mejoras. Incluye la migración de `npm` a `pnpm` en `tauri.conf.json`. |
| [`sesiones/LOGS-CAPTURADOS.md`](./sesiones/LOGS-CAPTURADOS.md) | Sesión 2025-10-10: logs crudos de la verificación de arranque de la app. |
| [`sesiones/SESION-COMPILACION-INSTALACION.md`](./sesiones/SESION-COMPILACION-INSTALACION.md) | Sesión 2025-01-27: compilación e instalación en Manjaro Linux. |
| [`sesiones/SESION-EDITAR-PROYECTOS.md`](./sesiones/SESION-EDITAR-PROYECTOS.md) | Sesión 2025-01-27: corrección de la edición de proyectos y optimización de rsync. |
| [`sesiones/SESION-ENLACES-MULTIPLES.md`](./sesiones/SESION-ENLACES-MULTIPLES.md) | Sesión 2025-10-11: implementación de enlaces múltiples por proyecto. |
| [`sesiones/SESION_RESUMEN.md`](./sesiones/SESION_RESUMEN.md) | Sesión 2025-10-19: cierre de v0.2.0, sistema multiplataforma. |

## `docs/historico/` — congelado, no refleja el proceso actual

Documentación de entregas viejas (mayormente el ciclo de Windows de v0.1.0 a
v0.3.0). **No la sigas como si fuera el proceso vigente.** Se conserva como
registro.

| Documento | Qué es |
| --------- | ------ |
| [`historico/win10.md`](./historico/win10.md) | Qué era el directorio `win10/`, qué contenía, por qué se sacó del índice de git y dónde vive ahora. **Leelo si buscás el paquete de Windows.** |
| [`historico/INDICE_DOCUMENTACION.md`](./historico/INDICE_DOCUMENTACION.md) | Índice viejo de documentación, orientado a Windows v0.3.0. Reemplazado por este archivo. |
| [`historico/LEEME_PRIMERO.md`](./historico/LEEME_PRIMERO.md) | Punto de entrada del ciclo de build para Windows v0.3.0. |
| [`historico/CHECKLIST_BUILD.md`](./historico/CHECKLIST_BUILD.md) | Checklist paso a paso del build para Windows. **Congelado en v0.3.0.** |
| [`historico/INSTRUCCIONES_BUILD_WINDOWS.md`](./historico/INSTRUCCIONES_BUILD_WINDOWS.md) | Instrucciones rápidas para compilar v0.3.0 en Windows con instaladores MSI y NSIS. |
| [`historico/INSTALACION_WINDOWS.md`](./historico/INSTALACION_WINDOWS.md) | Cómo instalar el toolchain (Rust, pnpm, VS Build Tools) en Windows. |
| [`historico/BUILD_WINDOWS.md`](./historico/BUILD_WINDOWS.md) | Cross-compilación a Windows desde Linux con el target `x86_64-pc-windows-gnu`. |
| [`historico/RESUMEN_BUILD_WINDOWS.md`](./historico/RESUMEN_BUILD_WINDOWS.md) | Resumen ejecutivo del build completo para Windows, con tiempos estimados. |
| [`historico/README_WINDOWS.md`](./historico/README_WINDOWS.md) | README de la entrega de Windows v0.3.0. |
| [`historico/MANUAL_USUARIO_WINDOWS.md`](./historico/MANUAL_USUARIO_WINDOWS.md) | Manual de usuario final de la app en Windows 10/11, v0.3.0. |
| [`historico/BUILD_SUMMARY.md`](./historico/BUILD_SUMMARY.md) | Reporte del build exitoso de v0.1.0 (2025-10-12). |
| [`historico/INSTALACION.md`](./historico/INSTALACION.md) | Instalación en Manjaro Linux desde los paquetes `.deb` / `.AppImage` generados. |
| [`historico/INSTALADO.md`](./historico/INSTALADO.md) | Confirmación y verificación post-instalación en Manjaro. |

---

## Qué NO está acá

- **`docs/historico/win10/`** — el paquete de distribución de Windows existe en
  disco pero está fuera del índice de git. Ver
  [`historico/win10.md`](./historico/win10.md).
- **`_archivo_local/`** — artefactos binarios sacados del índice (tarball de
  release, reportes de Playwright, PDFs de prueba). Están en disco, ignorados
  por git.
