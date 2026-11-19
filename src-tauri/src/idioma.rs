//! Idioma de la interfaz (#9): español si Windows tiene la interfaz en cualquier `es-*`, inglés en otro caso.
//! La regla es pura y se prueba sin Windows; solo `idioma_sistema` pregunta al sistema.

use serde::Serialize;

/// Idiomas en los que hay textos. Se serializa como la etiqueta que usan los ficheros de `src/textos/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Idioma {
    Es,
    En,
}

/// Elige el idioma a partir de una etiqueta BCP 47 (`es-ES`, `en_US`…). Solo mira el idioma principal, sin
/// distinguir mayúsculas; sin etiqueta o con una que no reconoce, inglés.
pub fn idioma_de(etiqueta: Option<&str>) -> Idioma {
    let principal = etiqueta
        .and_then(|e| e.split(['-', '_']).next())
        .unwrap_or_default();
    if principal.eq_ignore_ascii_case("es") {
        Idioma::Es
    } else {
        Idioma::En
    }
}

/// Idioma de la interfaz de Windows. `sys-locale` lo lee con `GetUserPreferredUILanguages`, la misma fuente que
/// `CultureInfo.CurrentUICulture`: es el idioma de la interfaz, no el formato regional.
pub fn idioma_sistema() -> Idioma {
    idioma_de(sys_locale::get_locale().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cualquier_espanol_es_espanol() {
        assert_eq!(idioma_de(Some("es-ES")), Idioma::Es);
        assert_eq!(idioma_de(Some("es-MX")), Idioma::Es);
        assert_eq!(idioma_de(Some("ES_es")), Idioma::Es);
        assert_eq!(idioma_de(Some("es")), Idioma::Es);
    }

    #[test]
    fn el_resto_es_ingles() {
        assert_eq!(idioma_de(Some("en-US")), Idioma::En);
        assert_eq!(idioma_de(Some("ca-ES")), Idioma::En);
        assert_eq!(idioma_de(Some("est-EE")), Idioma::En);
    }

    #[test]
    fn sin_etiqueta_o_con_basura_es_ingles() {
        assert_eq!(idioma_de(None), Idioma::En);
        assert_eq!(idioma_de(Some("")), Idioma::En);
        assert_eq!(idioma_de(Some("-")), Idioma::En);
    }

    #[test]
    fn se_serializa_como_la_etiqueta_de_los_ficheros_de_textos() {
        assert_eq!(serde_json::to_value(Idioma::Es).unwrap(), "es");
        assert_eq!(serde_json::to_value(Idioma::En).unwrap(), "en");
    }

    #[test]
    fn cada_idioma_tiene_su_fichero_de_textos_en_la_interfaz() {
        for idioma in [Idioma::Es, Idioma::En] {
            let etiqueta = serde_json::to_value(idioma).unwrap();
            let ruta = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../src/textos")
                .join(format!("{}.json", etiqueta.as_str().unwrap()));
            assert!(ruta.is_file(), "falta {}", ruta.display());
        }
    }
}
