use crate::db::Database;
use crate::models::project::Project;
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;

const FONT_SIZE_TITLE: f32 = 24.0;
const FONT_SIZE_SUBTITLE: f32 = 18.0;
const FONT_SIZE_BODY: f32 = 11.0;
const FONT_SIZE_SMALL: f32 = 9.0;

const COLOR_PRIMARY: (f32, f32, f32) = (0.2, 0.4, 0.6); // Azul
const COLOR_TEXT: (f32, f32, f32) = (0.1, 0.1, 0.1); // Negro suave
const COLOR_GRAY: (f32, f32, f32) = (0.5, 0.5, 0.5); // Gris

// Geometría de página (A4) y del cursor vertical.
const PAGE_WIDTH_MM: f32 = 210.0;
const PAGE_HEIGHT_MM: f32 = 297.0;
/// Y donde arranca el contenido de cada página. El cursor DECRECE desde acá.
const CURSOR_TOP_MM: f32 = 270.0;
/// Margen inferior: por debajo de esta Y no se escribe nada.
const CURSOR_BOTTOM_MM: f32 = 30.0;
/// Ancho de línea para el wrap de notas, en caracteres.
const NOTES_MAX_CHARS_PER_LINE: usize = 80;

/// Qué hacer con el cursor vertical antes de escribir un bloque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CursorAction {
    /// El bloque entra en la página actual.
    Continue,
    /// No entra: hay que abrir una página nueva.
    NewPage,
}

/// ¿Entra un bloque de alto `needed_mm` escribiendo desde `y_mm` hacia abajo?
///
/// Función PURA a propósito: es la decisión que antes estaba incrustada como
/// `if y_position.0 < 30.0 { break; }` en dos bucles, con un comentario que decía
/// "Nueva página si nos quedamos sin espacio" al lado de un `break` que jamás creó
/// ninguna página. El contenido restante se descartaba en silencio y el comando
/// devolvía Ok, así que el usuario recibía un PDF truncado que parecía completo.
///
/// El bloque ocupa de `y_mm` hasta `y_mm - needed_mm`. Entra si ese borde inferior no
/// perfora el margen; caer EXACTAMENTE sobre el margen todavía entra.
fn cursor_action(y_mm: f32, needed_mm: f32) -> CursorAction {
    if y_mm - needed_mm < CURSOR_BOTTOM_MM {
        CursorAction::NewPage
    } else {
        CursorAction::Continue
    }
}

/// Cursor vertical con estado: sabe en qué layer está escribiendo y abre una página
/// nueva cuando el contenido no entra. Encapsula el par
/// `doc.add_page(...)` + `get_page().get_layer()` para que los bucles de contenido no
/// tengan que conocer esa mecánica.
struct PdfCursor<'a> {
    doc: &'a PdfDocumentReference,
    layer: PdfLayerReference,
    y: Mm,
}

impl<'a> PdfCursor<'a> {
    fn new(doc: &'a PdfDocumentReference, layer: PdfLayerReference, y: Mm) -> Self {
        Self { doc, layer, y }
    }

    /// Garantiza espacio para un bloque de `needed_mm` y devuelve el layer VIGENTE.
    /// Si hubo que saltar de página, el layer devuelto es el de la página nueva y el
    /// cursor vuelve al tope. Devuelve el layer por valor (no por referencia) para no
    /// atar el préstamo y poder seguir llamando métodos `&mut self` del cursor.
    fn ensure_space(&mut self, needed_mm: f32) -> PdfLayerReference {
        if cursor_action(self.y.0, needed_mm) == CursorAction::NewPage {
            let (page, layer) =
                self.doc
                    .add_page(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), "Layer");
            self.layer = self.doc.get_page(page).get_layer(layer);
            self.y = Mm(CURSOR_TOP_MM);
        }
        self.layer.clone()
    }

    fn y(&self) -> Mm {
        self.y
    }

    fn advance(&mut self, mm: f32) {
        self.y -= Mm(mm);
    }
}

/// Corta `text` en líneas de a lo sumo `max_chars` caracteres, partiendo en el último
/// espacio disponible en vez de a ciegas a mitad de palabra.
///
/// El cálculo exacto requeriría medir el texto con las métricas de la fuente; esto es
/// la aproximación barata por conteo de caracteres, que es lo que ya hacía el código
/// anterior —sólo que aquél cortaba con `chunks(80)` y partía las palabras al medio—.
///
/// Respeta los saltos de línea del original (las notas se editan en un MarkdownEditor,
/// así que los párrafos son intencionales). Una palabra más larga que `max_chars` se
/// parte a la fuerza: no hay nada mejor que hacer con ella.
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();

    for raw_line in text.lines() {
        if raw_line.trim().is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();

        for word in raw_line.split_whitespace() {
            let word_len = word.chars().count();

            // Palabra sola más larga que el límite: se corta a la fuerza.
            if word_len > max_chars {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                let chars: Vec<char> = word.chars().collect();
                for chunk in chars.chunks(max_chars) {
                    lines.push(chunk.iter().collect());
                }
                continue;
            }

            let separator = if current.is_empty() { 0 } else { 1 };
            if current.chars().count() + separator + word_len > max_chars {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
            } else {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            }
        }

        if !current.is_empty() {
            lines.push(current);
        }
    }

    lines
}

fn rgb(color: (f32, f32, f32)) -> Color {
    Color::Rgb(Rgb::new(color.0, color.1, color.2, None))
}

pub fn export_project_to_pdf(
    db: &Database,
    project: &Project,
    output_path: &str,
) -> Result<(), String> {
    // Crear documento PDF (A4)
    let (doc, page1, layer1) = PdfDocument::new(
        "Proyecto: ".to_owned() + &project.name,
        Mm(PAGE_WIDTH_MM),
        Mm(PAGE_HEIGHT_MM),
        "Layer 1",
    );

    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Cargar fuente built-in (Helvetica)
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| e.to_string())?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| e.to_string())?;

    let mut y_position = Mm(CURSOR_TOP_MM); // Comenzar desde arriba

    // === PORTADA ===
    // Esta sección es de tamaño acotado (nunca desborda la primera página), así que
    // escribe directo sobre `current_layer` sin pasar por el cursor.
    current_layer.use_text(
        &project.name,
        FONT_SIZE_TITLE,
        Mm(20.0),
        y_position,
        &font_bold,
    );
    y_position -= Mm(10.0);

    // Descripción
    if !project.description.is_empty() {
        current_layer.set_fill_color(rgb(COLOR_GRAY));
        current_layer.use_text(&project.description, FONT_SIZE_BODY, Mm(20.0), y_position, &font);
        y_position -= Mm(8.0);
    }

    // Estado
    if let Some(status) = &project.status {
        current_layer.set_fill_color(rgb(COLOR_PRIMARY));
        let status_text = format!("Estado: {}", status.to_uppercase());
        current_layer.use_text(&status_text, FONT_SIZE_BODY, Mm(20.0), y_position, &font_bold);
        y_position -= Mm(8.0);
    }

    // Fecha de exportación
    current_layer.set_fill_color(rgb(COLOR_GRAY));
    let export_date = chrono::Local::now().format("%d/%m/%Y %H:%M").to_string();
    let date_text = format!("Exportado: {}", export_date);
    current_layer.use_text(&date_text, FONT_SIZE_SMALL, Mm(20.0), y_position, &font);
    y_position -= Mm(15.0);

    // === INFORMACIÓN GENERAL ===
    current_layer.set_fill_color(rgb(COLOR_PRIMARY));
    current_layer.use_text(
        "INFORMACIÓN GENERAL",
        FONT_SIZE_SUBTITLE,
        Mm(20.0),
        y_position,
        &font_bold,
    );
    y_position -= Mm(8.0);

    // Path local
    current_layer.set_fill_color(rgb(COLOR_TEXT));
    let path_text = format!("Path: {}", project.local_path);
    current_layer.use_text(&path_text, FONT_SIZE_BODY, Mm(20.0), y_position, &font);
    y_position -= Mm(6.0);

    // NOTA: acá había una sección "Tags" que imprimía `project.notes`. Era el MISMO
    // campo que la sección "NOTAS" del final, con otro nombre: el contenido salía
    // duplicado y encima mal etiquetado. `Project` no tiene campo `tags` (el que sí
    // lo tiene es `JournalEntry`), así que la sección "Tags" era la equivocada.

    // Grupo padre
    if let Some(parent_id) = project.parent_id {
        let parent_text = format!("Grupo padre: ID {}", parent_id);
        current_layer.use_text(&parent_text, FONT_SIZE_BODY, Mm(20.0), y_position, &font);
        y_position -= Mm(6.0);
    }

    y_position -= Mm(10.0);

    // A partir de acá el contenido es de largo ARBITRARIO (enlaces y notas), así que
    // pasa por el cursor, que abre páginas nuevas en vez de descartar lo que no entra.
    let mut cursor = PdfCursor::new(&doc, current_layer.clone(), y_position);

    // === ENLACES (CLICKEABLES) ===
    // Se reserva el alto del encabezado MÁS una primera entrada, para no dejar el
    // título huérfano al pie de una página.
    let layer = cursor.ensure_space(8.0 + 12.0);
    layer.set_fill_color(rgb(COLOR_PRIMARY));
    layer.use_text(
        "ENLACES IMPORTANTES",
        FONT_SIZE_SUBTITLE,
        Mm(20.0),
        cursor.y(),
        &font_bold,
    );
    cursor.advance(8.0);

    // Obtener enlaces desde la base de datos
    match db.get_project_links(project.id) {
        Ok(links) => {
            if links.is_empty() {
                let layer = cursor.ensure_space(6.0);
                layer.set_fill_color(rgb(COLOR_GRAY));
                layer.use_text(
                    "Sin enlaces guardados",
                    FONT_SIZE_BODY,
                    Mm(20.0),
                    cursor.y(),
                    &font,
                );
                cursor.advance(6.0);
            } else {
                for link in links {
                    // Cada entrada son DOS líneas (tipo + URL): 5.0 + 7.0 mm. Se
                    // reserva el bloque entero para no partir una entrada al medio.
                    let layer = cursor.ensure_space(5.0 + 7.0);

                    // Tipo de enlace en color normal
                    layer.set_fill_color(rgb(COLOR_TEXT));
                    let link_type_text = format!("• {} →", link.link_type);
                    layer.use_text(&link_type_text, FONT_SIZE_BODY, Mm(20.0), cursor.y(), &font_bold);
                    cursor.advance(5.0);

                    // URL en azul (una línea abajo)
                    layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.2, 0.8, None)));
                    layer.use_text(&link.url, FONT_SIZE_BODY, Mm(25.0), cursor.y(), &font);
                    cursor.advance(7.0);
                }
            }
        }
        Err(e) => {
            let layer = cursor.ensure_space(6.0);
            layer.set_fill_color(Color::Rgb(Rgb::new(0.8, 0.0, 0.0, None)));
            let error_text = format!("Error al cargar enlaces: {}", e);
            layer.use_text(&error_text, FONT_SIZE_SMALL, Mm(20.0), cursor.y(), &font);
            cursor.advance(6.0);
        }
    }

    cursor.advance(10.0);

    // === NOTAS ===
    if let Some(notes) = &project.notes {
        if !notes.is_empty() {
            let layer = cursor.ensure_space(8.0 + 5.0);
            layer.set_fill_color(rgb(COLOR_PRIMARY));
            layer.use_text("NOTAS", FONT_SIZE_SUBTITLE, Mm(20.0), cursor.y(), &font_bold);
            cursor.advance(8.0);

            for line in wrap_text(notes, NOTES_MAX_CHARS_PER_LINE) {
                let layer = cursor.ensure_space(5.0);
                layer.set_fill_color(rgb(COLOR_TEXT));
                layer.use_text(&line, FONT_SIZE_BODY, Mm(20.0), cursor.y(), &font);
                cursor.advance(5.0);
            }
        }
    }

    // Guardar PDF
    doc.save(&mut BufWriter::new(
        File::create(output_path).map_err(|e| format!("Error creando archivo: {}", e))?,
    ))
    .map_err(|e| format!("Error guardando PDF: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::{CreateLinkDTO, CreateProjectDTO};
    use std::path::PathBuf;
    use tempfile::TempDir;

    // ==================== cursor_action (función pura) ====================

    #[test]
    fn cursor_continues_when_the_block_fits() {
        assert_eq!(cursor_action(270.0, 12.0), CursorAction::Continue);
        assert_eq!(cursor_action(100.0, 5.0), CursorAction::Continue);
    }

    #[test]
    fn cursor_opens_a_new_page_when_the_block_does_not_fit() {
        assert_eq!(cursor_action(31.0, 5.0), CursorAction::NewPage);
        assert_eq!(cursor_action(0.0, 5.0), CursorAction::NewPage);
    }

    /// EL borde exacto. Los off-by-one viven acá, así que se assertan los tres puntos
    /// alrededor del margen en vez de dejarlos implícitos.
    #[test]
    fn cursor_boundary_at_the_bottom_margin_is_exact() {
        // Aterrizar JUSTO sobre el margen todavía entra: 35 - 5 == 30.0
        assert_eq!(
            cursor_action(CURSOR_BOTTOM_MM + 5.0, 5.0),
            CursorAction::Continue,
            "caer exactamente sobre el margen inferior debe entrar"
        );

        // Un pelo por debajo NO entra: 34.9 - 5 == 29.9
        assert_eq!(
            cursor_action(CURSOR_BOTTOM_MM + 4.9, 5.0),
            CursorAction::NewPage,
            "perforar el margen aunque sea por 0.1mm debe abrir página nueva"
        );

        // Estar parado EN el margen y querer escribir algo: no entra.
        assert_eq!(
            cursor_action(CURSOR_BOTTOM_MM, 5.0),
            CursorAction::NewPage,
            "y == 30.0 con contenido pendiente debe abrir página nueva"
        );

        // Caso degenerado: parado en el margen sin nada que escribir.
        assert_eq!(cursor_action(CURSOR_BOTTOM_MM, 0.0), CursorAction::Continue);
    }

    /// El bug original: `if y < 30.0 { break }` descartaba contenido. Con la función
    /// pura, ninguna Y por encima del margen puede terminar en descarte.
    #[test]
    fn no_position_above_the_margin_silently_discards_content() {
        let mut y = CURSOR_TOP_MM;
        let mut paginas = 1;
        // Simula 200 entradas de 12mm: el cursor debe repaginar, nunca "terminarse".
        for _ in 0..200 {
            if cursor_action(y, 12.0) == CursorAction::NewPage {
                paginas += 1;
                y = CURSOR_TOP_MM;
            }
            y -= 12.0;
        }
        assert!(
            paginas > 1,
            "200 entradas de 12mm no pueden caber en una sola página"
        );
        assert!(y >= CURSOR_BOTTOM_MM - 12.0, "el cursor nunca se va al infinito");
    }

    // ==================== wrap_text ====================

    #[test]
    fn wrap_does_not_split_a_word_that_fits_on_the_next_line() {
        // "extraordinario" (14) no entra en lo que queda de la primera línea de 20,
        // así que baja ENTERA en vez de partirse al medio como hacía chunks(80).
        let lines = wrap_text("un texto corto y extraordinario", 20);

        assert!(
            lines.iter().any(|l| l.contains("extraordinario")),
            "la palabra debe aparecer entera en alguna línea: {lines:?}"
        );
        for line in &lines {
            assert!(
                line.chars().count() <= 20,
                "ninguna línea debe exceder el límite: '{line}'"
            );
        }
    }

    #[test]
    fn wrap_force_splits_a_word_longer_than_the_limit() {
        let lines = wrap_text("aaaaaaaaaaaaaaaaaaaaaaaaa", 10);
        assert_eq!(lines, vec!["aaaaaaaaaa", "aaaaaaaaaa", "aaaaa"]);
    }

    #[test]
    fn wrap_preserves_paragraph_breaks() {
        let lines = wrap_text("primero\n\nsegundo", 40);
        assert_eq!(lines, vec!["primero", "", "segundo"]);
    }

    #[test]
    fn wrap_counts_characters_not_bytes() {
        // Los acentos ocupan 2 bytes en UTF-8: contar bytes partiría mal las líneas.
        let lines = wrap_text("árbol ñandú camión", 12);
        for line in &lines {
            assert!(
                line.chars().count() <= 12,
                "'{line}' mide {} caracteres",
                line.chars().count()
            );
        }
        assert!(lines.iter().any(|l| l.contains("ñandú")));
    }

    #[test]
    fn wrap_handles_empty_input() {
        assert!(wrap_text("", 80).is_empty());
    }

    // ==================== integración ====================

    fn test_db() -> Database {
        Database::new(PathBuf::from(":memory:")).expect("no se pudo crear la DB en memoria")
    }

    fn seed_project(db: &Database, name: &str, notes: Option<String>) -> Project {
        db.create_project(CreateProjectDTO {
            name: name.to_string(),
            description: "descripción de prueba".to_string(),
            local_path: "/tmp/proyecto".to_string(),
            documentation_url: None,
            ai_documentation_url: None,
            drive_link: None,
            notes,
            image_data: None,
            parent_id: None,
            group_color: None,
            group_icon: None,
        })
        .expect("no se pudo sembrar el proyecto")
    }

    fn seed_links(db: &Database, project_id: i64, cantidad: usize) {
        for i in 0..cantidad {
            db.create_link(CreateLinkDTO {
                project_id,
                link_type: "repository".to_string(),
                title: format!("Enlace {}", i),
                url: format!("https://example.com/repositorio/numero/{}", i),
            })
            .expect("no se pudo sembrar el enlace");
        }
    }

    /// Cuenta páginas en el PDF crudo, sin dependencia de lectura de PDF.
    ///
    /// `printpdf` 0.7 escribe los diccionarios de objeto SIN espacio (`/Type/Page`, no
    /// `/Type /Page`) y sin comprimir, así que se pueden contar leyendo los bytes.
    /// `/Type/Page` es prefijo de `/Type/Pages` (el nodo raíz del árbol), así que hay
    /// que restar esas ocurrencias.
    ///
    /// Verificado empíricamente contra un PDF generado por este módulo: 200 enlaces
    /// producen 11 páginas, y el conteo coincide con el de `/MediaBox`.
    fn count_pdf_pages(bytes: &[u8]) -> usize {
        let texto = String::from_utf8_lossy(bytes);
        let hojas = texto.match_indices("/Type/Page").count();
        let arboles = texto.match_indices("/Type/Pages").count();
        hojas - arboles
    }

    /// EL test del bug: con 200 enlaces el PDF tiene que contener los 200, no los que
    /// entraban en la primera página. Antes del fix el archivo de 200 enlaces pesaba
    /// prácticamente lo mismo que el de 2, porque el `break` tiraba el resto.
    #[test]
    fn exporting_many_links_does_not_discard_content() {
        let dir = TempDir::new().unwrap();
        let db = test_db();

        let pocos = seed_project(&db, "Pocos enlaces", None);
        seed_links(&db, pocos.id, 2);
        let path_pocos = dir.path().join("pocos.pdf");
        export_project_to_pdf(&db, &pocos, path_pocos.to_str().unwrap())
            .expect("la exportación con pocos enlaces no debería fallar");

        let muchos = seed_project(&db, "Muchos enlaces", None);
        seed_links(&db, muchos.id, 200);
        let path_muchos = dir.path().join("muchos.pdf");
        export_project_to_pdf(&db, &muchos, path_muchos.to_str().unwrap())
            .expect("la exportación con muchos enlaces no debería fallar");

        let tam_pocos = std::fs::metadata(&path_pocos).unwrap().len();
        let tam_muchos = std::fs::metadata(&path_muchos).unwrap().len();

        assert!(tam_pocos > 0, "el PDF de control no debería estar vacío");
        assert!(
            tam_muchos > tam_pocos * 2,
            "el PDF con 200 enlaces ({tam_muchos} bytes) debe ser sustancialmente más \
             grande que el de 2 ({tam_pocos} bytes); si son parecidos, el contenido se \
             está descartando igual que antes del fix"
        );

        // Aserción fuerte: 2 enlaces entran en una página, 200 no pueden.
        let bytes_pocos = std::fs::read(&path_pocos).unwrap();
        let bytes_muchos = std::fs::read(&path_muchos).unwrap();
        let paginas_pocos = count_pdf_pages(&bytes_pocos);
        let paginas_muchos = count_pdf_pages(&bytes_muchos);

        assert_eq!(
            paginas_pocos, 1,
            "2 enlaces deberían entrar en una sola página"
        );
        assert!(
            paginas_muchos > 1,
            "200 enlaces deben repaginar; el PDF quedó en {paginas_muchos} página(s), o sea \
             que el contenido sobrante se sigue descartando como antes del fix"
        );

        // Contraste independiente: cada página lleva su propio /MediaBox.
        let mediaboxes = String::from_utf8_lossy(&bytes_muchos)
            .match_indices("/MediaBox")
            .count();
        assert_eq!(
            mediaboxes, paginas_muchos,
            "el conteo por /Type/Page debe coincidir con el de /MediaBox"
        );
    }

    /// Mismo bug por el otro bucle: notas largas se truncaban al llegar al margen.
    #[test]
    fn exporting_long_notes_does_not_discard_content() {
        let dir = TempDir::new().unwrap();
        let db = test_db();

        let corto = seed_project(&db, "Notas cortas", Some("una nota breve".to_string()));
        let path_corto = dir.path().join("corto.pdf");
        export_project_to_pdf(&db, &corto, path_corto.to_str().unwrap()).unwrap();

        let notas_largas = "Esta es una línea de notas con bastante contenido real. ".repeat(300);
        let largo = seed_project(&db, "Notas largas", Some(notas_largas));
        let path_largo = dir.path().join("largo.pdf");
        export_project_to_pdf(&db, &largo, path_largo.to_str().unwrap()).unwrap();

        let tam_corto = std::fs::metadata(&path_corto).unwrap().len();
        let tam_largo = std::fs::metadata(&path_largo).unwrap().len();

        assert!(
            tam_largo > tam_corto * 2,
            "el PDF con notas largas ({tam_largo} bytes) debe ser sustancialmente más \
             grande que el de notas cortas ({tam_corto} bytes)"
        );

        let paginas_largo = count_pdf_pages(&std::fs::read(&path_largo).unwrap());
        assert!(
            paginas_largo > 1,
            "unas notas de 300 líneas deben repaginar; quedaron en {paginas_largo} página(s)"
        );
    }

    #[test]
    fn exporting_a_project_without_links_or_notes_still_produces_a_pdf() {
        let dir = TempDir::new().unwrap();
        let db = test_db();
        let vacio = seed_project(&db, "Sin nada", None);

        let path = dir.path().join("vacio.pdf");
        export_project_to_pdf(&db, &vacio, path.to_str().unwrap())
            .expect("un proyecto sin enlaces ni notas debe exportar igual");

        assert!(std::fs::metadata(&path).unwrap().len() > 0);
    }
}
