# `gestor` — la CLI para navegar tus proyectos

Herramienta **externa** a la app: un script de shell que lee la misma base
SQLite que usa la app gráfica y te deja saltar a la carpeta de cualquier
proyecto sin soltar la terminal.

No es parte de `src/` ni de `src-tauri/`. No se compila, no se empaqueta, no
toca el código de la app. Vive en `~/.local/bin/gestor` y este documento explica
cómo funciona y cómo reinstalarla.

---

## Las dos formas de usarla

### 1. Menú interactivo (finder)

```bash
gestor
```

Abre un menú `fzf` con los **proyectos activos** (los que están en la papelera
no aparecen). Navegás con las flechas o tipeás para filtrar, y con Enter caés
parado dentro de la carpeta del proyecto.

Cada fila muestra el nombre y, si el proyecto pertenece a un grupo, el nombre
del grupo padre después de un `·`. Los proyectos fijados llevan 📌. El panel de
preview de la derecha muestra grupo, ruta (con ✓ o ✗ según exista en disco) y
descripción.

`Esc` cancela: no se mueve nada y la CLI sale con código 130.

### 2. Comandos directos (sin menú)

```bash
gestor list                    # nombre<TAB>ruta, un proyecto por línea
gestor list --grupos           # agrupado visualmente por grupo padre
gestor list --con-papelera     # incluye también los borrados (alias: --todo)

gestor abrir CRM_Multas        # nombre exacto: va derecho
gestor abrir multas            # varios matches: abre el menú ya acotado
gestor abrir telwinet/speed    # también busca dentro de local_path

gestor ayuda                   # uso y ejemplos (alias: --help, -h)
```

`abrir` busca de forma parcial y sin distinguir mayúsculas, tanto en `name`
como en `local_path`. Si hay **un solo** resultado, salta directo. Si hay
varios pero **uno calza exacto por nombre**, gana ese sin preguntar. Si hay
varios ambiguos, te abre el menú limitado a esos candidatos. Si no hay
ninguno, sale con un error claro.

Un subcomando desconocido imprime la ayuda y sale con código `2`.

---

## Por qué hay una función de shell (y no alcanza con el script)

Esta es la parte importante, y es una ley del sistema operativo, no un capricho:

> **Un proceso hijo no puede cambiar el directorio de trabajo de su shell padre.**

Cuando ejecutás un script, el shell hace `fork` y el script corre en un proceso
aparte, con su propio directorio de trabajo. Si ese script hace `cd /donde/sea`,
se mueve **él**, y cuando termina, ese proceso muere y se lleva el cambio a la
tumba. Tu terminal sigue exactamente donde estaba. Por eso `cd` es un
*builtin* del shell y no un programa: tiene que correr **dentro** de tu shell
para servir de algo.

La solución es dividir las responsabilidades:

| Quién | Qué hace |
| ----- | -------- |
| `~/.local/bin/gestor` (script) | Consulta la base, dibuja el menú, **resuelve** la ruta del proyecto y la entrega. Nunca hace `cd`. |
| Función `gestor()` en tu shell | Ejecuta el script, lee la ruta que resolvió y hace el `cd`. Como es una función, corre **en tu shell**: el `cd` es efectivo. |

### Cómo se pasan la ruta

La función crea un archivo temporal y se lo pasa al script en la variable
`GESTOR_CD_FILE`:

```bash
gestor() {
  local __gestor_out __gestor_rc __gestor_dir
  __gestor_out="$(mktemp "${TMPDIR:-/tmp}/gestor-cd.XXXXXX" 2>/dev/null)" || {
    command gestor "$@"; return $?
  }
  GESTOR_CD_FILE="$__gestor_out" command gestor "$@"
  __gestor_rc=$?
  __gestor_dir=''
  [ -s "$__gestor_out" ] && __gestor_dir="$(cat "$__gestor_out")"
  rm -f "$__gestor_out"
  if [ -n "$__gestor_dir" ] && [ -d "$__gestor_dir" ]; then
    cd "$__gestor_dir" || return $?
  fi
  return $__gestor_rc
}
```

Si el archivo queda vacío —porque corriste `gestor list`, pediste la ayuda,
cancelaste el menú o la ruta no existe— la función no hace `cd`. Sin casos
especiales, sin listas de subcomandos que mantener: **si no hubo ruta, no hubo
viaje.**

### ¿Por qué un archivo y no `cd "$(gestor abrir x)"`?

Con sustitución de comandos, el `stdout` del script queda capturado. Eso obliga
a que *todo* lo que el script imprime sea la ruta y nada más, y complica los
mensajes, el menú y el preview de `fzf`, que necesitan salir al terminal. Con el
archivo de estado, el script conserva su `stdout` y su `stderr` normales: la UI
sale limpia y la ruta viaja por un canal aparte.

De todos modos **las dos formas funcionan**. Si invocás el script directo, sin
la función (por ejemplo desde otro script), imprime la ruta cruda por `stdout`:

```bash
cd "$(command gestor abrir multas)"
```

---

## Instalación

### Requisitos

| Herramienta | Obligatoria | Para qué |
| ----------- | ----------- | -------- |
| `sqlite3` | Sí | Leer la base de proyectos. |
| `fzf` | No | El menú interactivo. Sin `fzf`, degrada a un prompt numerado. |

La app gráfica **no** necesita estar corriendo: la CLI lee el archivo de base
directamente.

### 1. El script

Va en `~/.local/bin/gestor`, con permiso de ejecución:

```bash
chmod +x ~/.local/bin/gestor
```

Asegurate de que `~/.local/bin` esté en tu `PATH`.

### 2. La función de shell

Se agrega al final de `~/.bashrc` **y** de `~/.zshrc`, entre marcas:

```
# >>> gestor-proyectos cli >>>
...la función...
# <<< gestor-proyectos cli <<<
```

Las marcas hacen que la instalación sea idempotente y que desinstalar sea
borrar el bloque completo. La función es compatible con bash y con zsh sin
cambios.

Después, recargá: `source ~/.zshrc` (o abrí una terminal nueva).

---

## Cómo trata la base de datos

La CLI usa la **misma** base que la app:

```
~/.local/share/gestor-proyectos/projects.db
```

Se puede apuntar a otra con la variable `GESTOR_DB`.

- **Todas las lecturas son read-only**, vía URI: `file:$DB?mode=ro`. La CLI no
  puede corromper ni bloquear la base de la app aunque quiera.
- **La única escritura** es el registro del "abierto", que replica lo que hace
  la app para su lista de recientes:

  ```sql
  PRAGMA busy_timeout=2000;
  BEGIN IMMEDIATE;
  UPDATE projects
     SET opened_count = COALESCE(opened_count,0) + 1,
         last_opened_at = datetime('now')
   WHERE id = ?;
  COMMIT;
  ```

  Transacción corta y `busy_timeout` bajo: si la app tiene la base tomada, la
  CLI espera un toque y, si no puede, **avisa por `stderr` y sigue igual**. Que
  falle el contador nunca te impide llegar a tu carpeta.

- Los proyectos en la papelera (`deleted_at IS NOT NULL`) quedan fuera de todo,
  salvo que pidas `list --con-papelera`.

### Cuando la carpeta ya no existe

Si el proyecto apunta a una ruta que se movió o se borró, la CLI **no hace
`cd`**. Avisa por `stderr` y sale con código 1:

```
gestor: la carpeta de 'CRM_Multas' no existe: /home/2025
gestor: el proyecto se movió o se borró — arreglá la ruta en la app gráfica.
```

La CLI no corrige rutas: eso se arregla en la app, que es la dueña del dato.

---

## Detalles de implementación que conviene saber

### El separador interno es `US` (0x1f), no TAB

`sqlite3` emite las filas con un separador de campos, y el script las parsea con
`IFS=... read`. **En bash, el TAB es un caracter IFS-whitespace**: dos
delimitadores seguidos colapsan en uno solo. Con TAB, un proyecto sin grupo
(campo vacío) perdía una columna y todo el resto se corría un lugar — la lista
mostraba la *descripción* donde debía ir la *ruta*.

`US` (0x1f, *unit separator*) no es whitespace, así que nunca colapsa y los
campos vacíos se respetan. La consulta SQL además reemplaza cualquier `US`,
TAB o newline que pudiera venir dentro de un campo.

El TAB sigue siendo el separador de la **salida** de `gestor list`, que es lo
que vas a querer cortar con `cut -f2` o `awk`.

### Degradación sin `fzf`

Si `fzf` no está instalado —o si exportás `FZF_DISABLED=1`— el menú cae a un
prompt numerado que no depende de nada:

```
proyecto
   1)  📌 gestor_de_proyectos
   2)  📌 Proxmox_Speedtest  ·  Telwinet
   ...
Número (Enter para cancelar):
```

Valida el rango, rechaza lo no numérico y Enter vacío cancela. Lee del terminal
de control cuando `stdin` está redirigido, y cae a `stdin` si no hay terminal,
sin romperse en scripts.

### Inyección SQL

El texto que pasás a `abrir` se escapa antes de entrar a la consulta: comillas
simples duplicadas y comodines de `LIKE` (`%`, `_`, `\`) neutralizados con
`ESCAPE`. Los ids se fuerzan a entero con expansión aritmética.

---

## Códigos de salida

| Código | Significado |
| ------ | ----------- |
| `0` | Todo bien. |
| `1` | Error de uso o de datos: sin coincidencias, falta un argumento, la carpeta no existe. |
| `2` | Subcomando o flag desconocidos (se imprime la ayuda). |
| `130` | Cancelaste el menú (Esc o Enter vacío). |
