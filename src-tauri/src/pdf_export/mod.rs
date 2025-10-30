use crate::db::Database;
use crate::models::project::Project;
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;

const FONT_SIZE_TITLE: f32 = 24.0;
const FONT_SIZE_SUBTITLE: f32 = 18.0;
const FONT_SIZE_HEADING: f32 = 14.0;
const FONT_SIZE_BODY: f32 = 11.0;
const FONT_SIZE_SMALL: f32 = 9.0;

const COLOR_PRIMARY: (f32, f32, f32) = (0.2, 0.4, 0.6); // Azul
const COLOR_TEXT: (f32, f32, f32) = (0.1, 0.1, 0.1); // Negro suave
const COLOR_GRAY: (f32, f32, f32) = (0.5, 0.5, 0.5); // Gris

pub fn export_project_to_pdf(
    db: &Database,
    project: &Project,
    output_path: &str,
) -> Result<(), String> {
    // Crear documento PDF (A4)
    let (doc, page1, layer1) =
        PdfDocument::new("Proyecto: ".to_owned() + &project.name, Mm(210.0), Mm(297.0), "Layer 1");

    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Cargar fuente built-in (Helvetica)
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| e.to_string())?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| e.to_string())?;

    let mut y_position = Mm(270.0); // Comenzar desde arriba

    // === PORTADA ===
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
        current_layer.set_fill_color(Color::Rgb(Rgb::new(
            COLOR_GRAY.0,
            COLOR_GRAY.1,
            COLOR_GRAY.2,
            None,
        )));
        current_layer.use_text(&project.description, FONT_SIZE_BODY, Mm(20.0), y_position, &font);
        y_position -= Mm(8.0);
    }

    // Estado
    if let Some(status) = &project.status {
        current_layer.set_fill_color(Color::Rgb(Rgb::new(
            COLOR_PRIMARY.0,
            COLOR_PRIMARY.1,
            COLOR_PRIMARY.2,
            None,
        )));
        let status_text = format!("Estado: {}", status.to_uppercase());
        current_layer.use_text(&status_text, FONT_SIZE_BODY, Mm(20.0), y_position, &font_bold);
        y_position -= Mm(8.0);
    }

    // Fecha de exportación
    current_layer.set_fill_color(Color::Rgb(Rgb::new(
        COLOR_GRAY.0,
        COLOR_GRAY.1,
        COLOR_GRAY.2,
        None,
    )));
    let export_date = chrono::Local::now().format("%d/%m/%Y %H:%M").to_string();
    let date_text = format!("Exportado: {}", export_date);
    current_layer.use_text(&date_text, FONT_SIZE_SMALL, Mm(20.0), y_position, &font);
    y_position -= Mm(15.0);

    // === INFORMACIÓN GENERAL ===
    current_layer.set_fill_color(Color::Rgb(Rgb::new(
        COLOR_PRIMARY.0,
        COLOR_PRIMARY.1,
        COLOR_PRIMARY.2,
        None,
    )));
    current_layer.use_text(
        "INFORMACIÓN GENERAL",
        FONT_SIZE_SUBTITLE,
        Mm(20.0),
        y_position,
        &font_bold,
    );
    y_position -= Mm(8.0);

    // Path local
    current_layer.set_fill_color(Color::Rgb(Rgb::new(
        COLOR_TEXT.0,
        COLOR_TEXT.1,
        COLOR_TEXT.2,
        None,
    )));
    let path_text = format!("Path: {}", project.local_path);
    current_layer.use_text(&path_text, FONT_SIZE_BODY, Mm(20.0), y_position, &font);
    y_position -= Mm(6.0);

    // Tags
    if let Some(tags) = &project.notes {
        if !tags.is_empty() {
            let tags_text = format!("Tags: {}", tags);
            current_layer.use_text(&tags_text, FONT_SIZE_BODY, Mm(20.0), y_position, &font);
            y_position -= Mm(6.0);
        }
    }

    // Grupo padre
    if let Some(parent_id) = project.parent_id {
        let parent_text = format!("Grupo padre: ID {}", parent_id);
        current_layer.use_text(&parent_text, FONT_SIZE_BODY, Mm(20.0), y_position, &font);
        y_position -= Mm(6.0);
    }

    y_position -= Mm(10.0);

    // === ENLACES (CLICKEABLES) ===
    current_layer.set_fill_color(Color::Rgb(Rgb::new(
        COLOR_PRIMARY.0,
        COLOR_PRIMARY.1,
        COLOR_PRIMARY.2,
        None,
    )));
    current_layer.use_text(
        "ENLACES IMPORTANTES",
        FONT_SIZE_SUBTITLE,
        Mm(20.0),
        y_position,
        &font_bold,
    );
    y_position -= Mm(8.0);

    // Obtener enlaces desde la base de datos
    match db.get_project_links(project.id) {
        Ok(links) => {
            if links.is_empty() {
                current_layer.set_fill_color(Color::Rgb(Rgb::new(
                    COLOR_GRAY.0,
                    COLOR_GRAY.1,
                    COLOR_GRAY.2,
                    None,
                )));
                current_layer.use_text(
                    "Sin enlaces guardados",
                    FONT_SIZE_BODY,
                    Mm(20.0),
                    y_position,
                    &font,
                );
                y_position -= Mm(6.0);
            } else {
                for link in links {
                    // Tipo de enlace en color normal
                    current_layer.set_fill_color(Color::Rgb(Rgb::new(
                        COLOR_TEXT.0,
                        COLOR_TEXT.1,
                        COLOR_TEXT.2,
                        None,
                    )));
                    let link_type_text = format!("• {} →", link.link_type);
                    current_layer.use_text(&link_type_text, FONT_SIZE_BODY, Mm(20.0), y_position, &font_bold);
                    y_position -= Mm(5.0);

                    // URL en azul (una línea abajo)
                    current_layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.2, 0.8, None)));
                    current_layer.use_text(&link.url, FONT_SIZE_BODY, Mm(25.0), y_position, &font);
                    y_position -= Mm(7.0);

                    if y_position.0 < 30.0 {
                        // Nueva página si nos quedamos sin espacio
                        break;
                    }
                }
            }
        }
        Err(e) => {
            current_layer.set_fill_color(Color::Rgb(Rgb::new(0.8, 0.0, 0.0, None)));
            let error_text = format!("Error al cargar enlaces: {}", e);
            current_layer.use_text(&error_text, FONT_SIZE_SMALL, Mm(20.0), y_position, &font);
            y_position -= Mm(6.0);
        }
    }

    y_position -= Mm(10.0);

    // === NOTAS ===
    if let Some(notes) = &project.notes {
        if !notes.is_empty() {
            current_layer.set_fill_color(Color::Rgb(Rgb::new(
                COLOR_PRIMARY.0,
                COLOR_PRIMARY.1,
                COLOR_PRIMARY.2,
                None,
            )));
            current_layer.use_text("NOTAS", FONT_SIZE_SUBTITLE, Mm(20.0), y_position, &font_bold);
            y_position -= Mm(8.0);

            current_layer.set_fill_color(Color::Rgb(Rgb::new(
                COLOR_TEXT.0,
                COLOR_TEXT.1,
                COLOR_TEXT.2,
                None,
            )));

            // Dividir notas en líneas (wrap simple)
            let max_chars_per_line = 80;
            for chunk in notes.chars().collect::<Vec<char>>().chunks(max_chars_per_line) {
                let line: String = chunk.iter().collect();
                current_layer.use_text(&line, FONT_SIZE_BODY, Mm(20.0), y_position, &font);
                y_position -= Mm(5.0);

                if y_position.0 < 30.0 {
                    break;
                }
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
