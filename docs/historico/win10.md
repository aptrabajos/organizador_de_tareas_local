# `win10/` — paquete de distribución para Windows (archivado)

> **Estado: archivado.** El directorio ya no está en el índice de git.
> Sigue existiendo en disco, en `docs/historico/win10/`.

## Qué era

`win10/` era un **paquete de distribución autocontenido para Windows 10**: el
ejecutable ya compilado, una copia congelada del código fuente con el que se
compiló, scripts de build y su propia documentación de instalación y uso.

No era código de la aplicación ni entraba en ningún build. Era una entrega,
commiteada tal cual quedó.

## Qué contenía (medición real, previa al archivado)

| Métrica | Valor medido |
| ------- | ------------ |
| Archivos trackeados en git | **120** |
| Archivos en disco | 121 (120 trackeados + `source/src-tauri/Cargo.lock`, que nunca estuvo trackeado) |
| Peso trackeado | **27.819.466 bytes ≈ 26,5 MB** |
| Peso en disco | ≈ 26,7 MB |

Los cinco archivos más pesados:

| Archivo | Bytes |
| ------- | ----- |
| `gestor-proyectos.exe` | 26.090.827 (≈ 24,9 MB) |
| `source/src-tauri/gen/schemas/linux-schema.json` | 404.626 |
| `source/src-tauri/gen/schemas/desktop-schema.json` | 404.626 |
| `source/src-tauri/gen/schemas/acl-manifests.json` | 138.102 |
| `source/src-tauri/Cargo.lock` | 137.385 |

Contenido de primer nivel:

- `gestor-proyectos.exe` — el binario de Windows (el 93 % del peso del directorio)
- `source/` — copia congelada del árbol de fuentes usado para esa compilación,
  con su propio `pnpm-lock.yaml`, su `Cargo.lock` y los `gen/schemas` de Tauri
- `scripts/` — scripts de empaquetado
- `docs/` — documentación de la entrega
- `DISTRIBUCION.md`, `EJECUTABLE_PORTABLE.md`, `QUICK_START.md`, `README.md`,
  `MANIFEST.txt`

## Por qué se sacó del índice

El repo trackeaba **70.984.558 bytes ≈ 67,7 MB** en 310 archivos.
`win10/` solo era **26,5 MB de esos 67,7 MB: el 39 % del peso del repo en
artefactos binarios y una copia duplicada del código fuente**.

Tres problemas concretos:

1. **Un `.exe` de 25 MB en git.** Git no sabe diffear un binario: cada
   recompilación que se hubiera commiteado habría sumado otros 25 MB a la
   historia, para siempre. Un clon fresco paga ese peso entero.
2. **`source/` es código duplicado.** Es una copia del mismo repo dentro del
   repo, con su propio `pnpm-lock.yaml`. Las herramientas la indexan, los greps
   la encuentran, y cualquiera puede editar el archivo equivocado sin darse
   cuenta. Para eso están los tags de git.
3. **Los artefactos de release no viven en el árbol de fuentes.** Van a GitHub
   Releases o a un registry de artefactos. El repo guarda cómo se construye la
   cosa, no la cosa construida.

Se archivó **completo, como una unidad**. No se separaron "los docs útiles" del
resto: la documentación de `win10/` describe esa entrega puntual, no el proceso
actual de build, y sacarla de contexto la volvería engañosa.

## Dónde vive ahora

En disco, en `docs/historico/win10/`, con todo su contenido intacto.
No se borró nada.

Ese path está listado en `.gitignore`, así que git lo ignora: los archivos
están ahí para quien los necesite, pero no pesan en el índice ni en los clones
nuevos.

## Nota importante: esto NO reduce el `.git`

El directorio `.git` de este repo pesa **61 MB**, y va a seguir pesando 61 MB
después de este cambio.

Sacar archivos del índice los saca de los *commits futuros*. Los blobs que ya
están en la historia siguen ahí: cualquier clon completo los baja igual.

Reducir el `.git` de verdad exige **reescribir la historia**
(`git filter-repo` o similar), lo que cambia todos los hashes de commit y obliga
a un force-push y a que cada persona vuelva a clonar. Eso es **una decisión
aparte, y no forma parte de este cambio.**

Lo que este archivado sí logra:

- El árbol de trabajo deja de tener 26,5 MB de binarios y código duplicado.
- Los commits futuros no arrastran artefactos de build.
- Las herramientas de búsqueda e indexado dejan de leer la copia de `source/`.
