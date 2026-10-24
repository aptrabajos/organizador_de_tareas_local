// Formato único de fechas y rangos de días locales.
//
// Toda fecha que se guarda en la base va en UTC con el formato
// `'%Y-%m-%d %H:%M:%S'`, el mismo que produce `CURRENT_TIMESTAMP` de SQLite. Los
// filtros por rango comparan texto, así que mezclar formatos (por ejemplo RFC3339)
// los rompería sin aviso: todo el código escribe con `formatear` y lee con
// `parsear`, y no hay otro camino.
//
// "Hoy" y "semana" son días calendario **locales**, convertidos a un rango
// `[desde, hasta)` en UTC antes de consultar. `rango_dia` y `rango_semana` reciben
// la zona como parámetro: `chrono::Local` aparece solo en el borde (los comandos) y
// los tests pasan zonas de `chrono-tz` sin tocar `TZ` del proceso.
//
// Excepción explícita: `projects.deleted_at`. `delete_project` lo escribe en hora
// **local** (`db/trash.rs`), la papelera lo muestra tal cual y sirve para
// identificar el lote borrado junto. Queda fuera de este módulo a propósito: nunca
// se compara con otra fecha ni entra en un rango, solo se pregunta
// `deleted_at IS NULL`. Pasarlo a UTC cambiaría lo que se ve en la papelera y
// obligaría a reescribir filas, sin ganar nada.

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, ParseError, TimeZone, Utc};

/// Formato con el que se guardan las fechas en la base: el de `CURRENT_TIMESTAMP`.
pub const FORMATO: &str = "%Y-%m-%d %H:%M:%S";

/// Intervalo `[desde, hasta)` en UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangoUtc {
    pub desde: DateTime<Utc>,
    pub hasta: DateTime<Utc>,
}

impl RangoUtc {
    /// `desde` en el formato de la base, para comparar contra columnas de texto.
    pub fn desde_texto(&self) -> String {
        formatear(self.desde)
    }

    /// `hasta` en el formato de la base, para comparar contra columnas de texto.
    pub fn hasta_texto(&self) -> String {
        formatear(self.hasta)
    }

    /// Duración del rango en segundos. Solo la usan los tests de zonas.
    #[cfg(test)]
    pub fn segundos(&self) -> i64 {
        (self.hasta - self.desde).num_seconds()
    }
}

/// Rango que contiene cualquier fecha de la base, para el total "desde siempre". Los
/// extremos se formatean con cuatro cifras de año, así que la comparación de texto
/// sigue valiendo.
pub fn rango_total() -> RangoUtc {
    RangoUtc {
        desde: DateTime::UNIX_EPOCH,
        hasta: NaiveDate::from_ymd_opt(9999, 12, 31)
            .and_then(|fecha| fecha.and_hms_opt(23, 59, 59))
            .map_or(DateTime::UNIX_EPOCH, |fin| fin.and_utc()),
    }
}

/// Formatea un instante para guardarlo en la base. Trunca los subsegundos (no
/// redondea): las columnas guardan segundos enteros.
pub fn formatear(instante: DateTime<Utc>) -> String {
    instante.format(FORMATO).to_string()
}

/// Lee una fecha guardada en la base (UTC, formato `FORMATO`). Rechaza cualquier
/// otro formato, RFC3339 incluido.
pub fn parsear(texto: &str) -> Result<DateTime<Utc>, ParseError> {
    NaiveDateTime::parse_from_str(texto, FORMATO).map(|n| n.and_utc())
}

/// Rango UTC del día local `fecha` en la zona `tz`: de su medianoche a la del día
/// siguiente. Mide 23 o 25 h los días de cambio de hora.
pub fn rango_dia<Tz: TimeZone>(tz: &Tz, fecha: NaiveDate) -> RangoUtc {
    RangoUtc {
        desde: inicio_dia(tz, fecha),
        hasta: inicio_dia(tz, siguiente(fecha)),
    }
}

/// Rango UTC de la semana que termina en `hoy`: los últimos 7 días calendario
/// locales, incluido hoy. Es `[inicio de (hoy − 6), inicio de (hoy + 1))`.
pub fn rango_semana<Tz: TimeZone>(tz: &Tz, hoy: NaiveDate) -> RangoUtc {
    RangoUtc {
        desde: inicio_dia(tz, hoy - Duration::days(6)),
        hasta: inicio_dia(tz, siguiente(hoy)),
    }
}

fn siguiente(fecha: NaiveDate) -> NaiveDate {
    fecha + Duration::days(1)
}

/// Primer instante del día local `fecha`, en UTC.
///
/// - Medianoche ambigua (el reloj vuelve atrás y pasa dos veces): la primera.
/// - Medianoche inexistente (el reloj salta hacia adelante a las 00:00): el día
///   empieza en la primera hora local que existe, buscando de a un minuto. Un
///   `unwrap` de `.earliest()` haría panic ahí.
fn inicio_dia<Tz: TimeZone>(tz: &Tz, fecha: NaiveDate) -> DateTime<Utc> {
    let medianoche = fecha.and_time(NaiveTime::MIN);
    // El salto más largo registrado es de un día entero (Pacific/Apia, 2011-12-30):
    // con 48 h de búsqueda alcanza para cualquier zona real. Si el día no existe,
    // empieza donde empieza el siguiente y su rango queda vacío.
    for minutos in 0..=48 * 60 {
        let local = medianoche + Duration::minutes(minutos);
        if let Some(instante) = tz.from_local_datetime(&local).earliest() {
            return instante.with_timezone(&Utc);
        }
    }
    // Inalcanzable con la base de zonas real: antes que un panic, la medianoche
    // tomada como UTC.
    medianoche.and_utc()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::LocalResult;
    use chrono_tz::America::{Argentina::Buenos_Aires, Havana, Santiago};
    use chrono_tz::Europe::Madrid;
    use chrono_tz::Pacific::Apia;
    use rusqlite::Connection;

    fn fecha(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn utc(texto: &str) -> DateTime<Utc> {
        parsear(texto).unwrap()
    }

    fn medianoche(f: NaiveDate) -> NaiveDateTime {
        f.and_time(NaiveTime::MIN)
    }

    // --- Formato y parser ---

    #[test]
    fn ida_y_vuelta_con_current_timestamp() {
        let conn = Connection::open_in_memory().unwrap();
        let texto: String = conn
            .query_row("SELECT CURRENT_TIMESTAMP", [], |r| r.get(0))
            .unwrap();
        let instante = parsear(&texto).unwrap();
        assert_eq!(formatear(instante), texto);
        // Y es UTC: queda a segundos del reloj de pared en UTC.
        assert!((Utc::now() - instante).num_seconds().abs() < 5);
    }

    #[test]
    fn sqlite_lee_lo_que_escribe_formatear() {
        let conn = Connection::open_in_memory().unwrap();
        let instante = utc("2025-03-09 05:00:00");
        // `datetime()` normaliza a su formato canónico: si devuelve lo mismo, SQLite
        // entiende el texto y las comparaciones con `CURRENT_TIMESTAMP` son válidas.
        let normalizado: String = conn
            .query_row("SELECT datetime(?1)", [formatear(instante)], |r| r.get(0))
            .unwrap();
        assert_eq!(normalizado, formatear(instante));
    }

    #[test]
    fn formatear_trunca_subsegundos() {
        let instante = utc("2025-01-01 10:00:00") + Duration::milliseconds(999);
        assert_eq!(formatear(instante), "2025-01-01 10:00:00");
    }

    #[test]
    fn parsear_rechaza_otros_formatos() {
        assert!(parsear("2025-01-01T10:00:00Z").is_err());
        assert!(parsear("2025-01-01T10:00:00+00:00").is_err());
        assert!(parsear("2025-01-01 10:00").is_err());
        assert!(parsear("2025-01-01").is_err());
    }

    #[test]
    fn el_orden_del_texto_es_el_orden_del_tiempo() {
        let a = utc("2025-09-30 23:59:59");
        let b = utc("2025-10-01 00:00:00");
        let c = utc("2025-10-10 00:00:00");
        assert!(formatear(a) < formatear(b));
        assert!(formatear(b) < formatear(c));
    }

    // --- Zonas (tabla de 5.1). Cada caso afirma primero su precondición. ---

    #[test]
    fn buenos_aires_el_dia_empieza_a_las_03_utc() {
        let f = fecha(2025, 10, 21);
        // Precondición: sin horario de verano, UTC−3 fijo.
        assert_eq!(
            Buenos_Aires
                .from_local_datetime(&medianoche(f))
                .single()
                .map(|d| d.with_timezone(&Utc)),
            Some(utc("2025-10-21 03:00:00"))
        );

        let r = rango_dia(&Buenos_Aires, f);
        assert_eq!(r.desde, utc("2025-10-21 03:00:00"));
        assert_eq!(r.hasta, utc("2025-10-22 03:00:00"));
        assert_eq!(r.segundos(), 24 * 3600);
        assert_eq!(r.desde_texto(), "2025-10-21 03:00:00");
        assert_eq!(r.hasta_texto(), "2025-10-22 03:00:00");
    }

    #[test]
    fn santiago_medianoche_inexistente_al_empezar_el_horario_de_verano() {
        let f = fecha(2025, 9, 7);
        // Precondición: el sábado a las 24:00 el reloj salta a la 01:00.
        assert_eq!(
            Santiago.from_local_datetime(&medianoche(f)),
            LocalResult::None
        );

        let r = rango_dia(&Santiago, f);
        // El día empieza a la 01:00 local (−03) = 04:00 UTC y dura 23 h.
        assert_eq!(r.desde, utc("2025-09-07 04:00:00"));
        assert_eq!(r.hasta, utc("2025-09-08 03:00:00"));
        assert_eq!(r.segundos(), 23 * 3600);

        // El día anterior termina donde empieza este: no se pierde ni se duplica nada.
        let anterior = rango_dia(&Santiago, fecha(2025, 9, 6));
        assert_eq!(anterior.hasta, r.desde);
        assert_eq!(anterior.segundos(), 24 * 3600);
    }

    #[test]
    fn havana_medianoche_inexistente_en_marzo() {
        let f = fecha(2025, 3, 9);
        // Precondición: a las 00:00 el reloj salta a la 01:00.
        assert_eq!(
            Havana.from_local_datetime(&medianoche(f)),
            LocalResult::None
        );

        let r = rango_dia(&Havana, f);
        // 01:00 local (−04) = 05:00 UTC; el día dura 23 h.
        assert_eq!(r.desde, utc("2025-03-09 05:00:00"));
        assert_eq!(r.hasta, utc("2025-03-10 04:00:00"));
        assert_eq!(r.segundos(), 23 * 3600);
        assert_eq!(rango_dia(&Havana, fecha(2025, 3, 8)).hasta, r.desde);
    }

    #[test]
    fn havana_medianoche_ambigua_en_noviembre() {
        let f = fecha(2025, 11, 2);
        // Precondición: a la 01:00 el reloj vuelve a las 00:00, que pasa dos veces.
        assert!(matches!(
            Havana.from_local_datetime(&medianoche(f)),
            LocalResult::Ambiguous(_, _)
        ));

        let r = rango_dia(&Havana, f);
        // Se toma la primera medianoche (−04) = 04:00 UTC; el día dura 25 h.
        assert_eq!(r.desde, utc("2025-11-02 04:00:00"));
        assert_eq!(r.hasta, utc("2025-11-03 05:00:00"));
        assert_eq!(r.segundos(), 25 * 3600);
        assert_eq!(rango_dia(&Havana, fecha(2025, 11, 1)).hasta, r.desde);
    }

    #[test]
    fn madrid_dias_de_23_y_25_horas_sin_tocar_la_medianoche() {
        let primavera = fecha(2025, 3, 30);
        let otono = fecha(2025, 10, 26);
        // Precondición: las dos medianoches existen y son únicas (el cambio es a las
        // 02:00 y a las 03:00), pero el día mide 23 y 25 h.
        assert!(Madrid
            .from_local_datetime(&medianoche(primavera))
            .single()
            .is_some());
        assert!(Madrid
            .from_local_datetime(&medianoche(otono))
            .single()
            .is_some());

        let r = rango_dia(&Madrid, primavera);
        assert_eq!(r.desde, utc("2025-03-29 23:00:00"));
        assert_eq!(r.segundos(), 23 * 3600);

        let r = rango_dia(&Madrid, otono);
        assert_eq!(r.desde, utc("2025-10-25 22:00:00"));
        assert_eq!(r.segundos(), 25 * 3600);
    }

    #[test]
    fn dia_que_no_existe_queda_vacio_sin_panic() {
        let f = fecha(2011, 12, 30);
        // Precondición: Samoa se salteó el 30 de diciembre de 2011 entero.
        for hora in [0, 12, 23] {
            let local = f.and_hms_opt(hora, 0, 0).unwrap();
            assert_eq!(Apia.from_local_datetime(&local), LocalResult::None);
        }

        let r = rango_dia(&Apia, f);
        assert_eq!(r.segundos(), 0);
        assert_eq!(rango_dia(&Apia, fecha(2011, 12, 29)).hasta, r.desde);
        assert_eq!(rango_dia(&Apia, fecha(2011, 12, 31)).desde, r.hasta);
    }

    // --- Total ---

    #[test]
    fn rango_total_se_compara_bien_como_texto() {
        let r = rango_total();
        assert_eq!(r.desde_texto(), "1970-01-01 00:00:00");
        assert_eq!(r.hasta_texto(), "9999-12-31 23:59:59");
        assert!(r.desde_texto().as_str() < "2026-10-20 09:00:00");
        assert!("2026-10-20 09:00:00" < r.hasta_texto().as_str());
    }

    // --- Semana ---

    #[test]
    fn semana_son_7_fechas_locales_incluido_hoy() {
        let hoy = fecha(2025, 10, 21);
        let r = rango_semana(&Buenos_Aires, hoy);
        assert_eq!(r.desde, rango_dia(&Buenos_Aires, fecha(2025, 10, 15)).desde);
        assert_eq!(r.hasta, rango_dia(&Buenos_Aires, hoy).hasta);
        assert_eq!(r.segundos(), 7 * 24 * 3600);
    }

    #[test]
    fn semana_con_cambio_de_hora_sigue_siendo_7_fechas() {
        // Incluye el día de 23 h de Santiago: la semana mide 7 × 24 − 1 h.
        let hoy = fecha(2025, 9, 10);
        let r = rango_semana(&Santiago, hoy);
        assert_eq!(r.desde, rango_dia(&Santiago, fecha(2025, 9, 4)).desde);
        assert_eq!(r.hasta, rango_dia(&Santiago, hoy).hasta);
        assert_eq!(r.segundos(), (7 * 24 - 1) * 3600);
    }
}
