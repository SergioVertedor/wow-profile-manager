//! Lector del `.toc` de un addon: las líneas `## Clave: valor` de su cabecera. Puro (texto → datos), sin disco.
//! El `.toc` es una pista, no la verdad (`docs/modelo-de-datos.md`): lo que falta queda vacío, nunca se inventa.

use serde::Serialize;
use std::collections::HashMap;

/// Lo que la app usa de un `.toc`. Medido sobre 226 `.toc` reales (spec 014).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Toc {
    /// `## Title` sin códigos de color (`|cAARRGGBB…|r`).
    pub titulo: Option<String>,
    pub version: Option<String>,
    /// `## X-Website`. Los identificadores de CurseForge o Wago no son enlaces.
    pub enlace: Option<String>,
    /// `## Dependencies` y `## RequiredDeps`, sin repetir. `## OptionalDeps` no es un requisito.
    pub dependencias: Vec<String>,
    /// `## SavedVariables`: variables de la cuenta.
    pub sv_cuenta: Vec<String>,
    /// `## SavedVariablesPerCharacter`: variables de cada personaje.
    pub sv_personaje: Vec<String>,
}

const BOM: char = '\u{feff}';
/// Largo del color en `|cAARRGGBB`.
const DIGITOS_COLOR: usize = 8;

pub fn leer(texto: &str) -> Toc {
    let campos = campos_de(texto);
    let texto_de = |clave: &str| {
        campos
            .get(clave)
            .map(|v| (*v).to_owned())
            .filter(|v| !v.is_empty())
    };
    let lista_de = |clave: &str| campos.get(clave).map_or_else(Vec::new, |v| lista(v));

    let mut dependencias = lista_de("dependencies");
    for dependencia in lista_de("requireddeps") {
        if !dependencias.contains(&dependencia) {
            dependencias.push(dependencia);
        }
    }
    Toc {
        titulo: texto_de("title")
            .map(|t| sin_colores(&t))
            .filter(|t| !t.is_empty()),
        version: texto_de("version"),
        enlace: texto_de("x-website"),
        dependencias,
        sv_cuenta: lista_de("savedvariables"),
        sv_personaje: lista_de("savedvariablespercharacter"),
    }
}

/// Claves en minúsculas (el juego no distingue mayúsculas) y valores recortados. Si una clave se repite, cuenta la
/// primera [I: no hay repetidas medidas]. Las líneas `##` sin `:` no son campos.
fn campos_de(texto: &str) -> HashMap<String, &str> {
    let mut campos = HashMap::new();
    // `lines()` también corta en `\r\n`.
    for linea in texto.trim_start_matches(BOM).lines() {
        let Some((clave, valor)) = linea
            .strip_prefix("##")
            .and_then(|resto| resto.split_once(':'))
        else {
            continue;
        };
        campos
            .entry(clave.trim().to_ascii_lowercase())
            .or_insert_with(|| valor.trim());
    }
    campos
}

/// `a, b,,c` → `[a, b, c]`. Un valor vacío es una lista vacía.
fn lista(valor: &str) -> Vec<String> {
    valor
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Quita `|cAARRGGBB` y `|r`. Cualquier otra `|` se deja tal cual.
fn sin_colores(texto: &str) -> String {
    let mut limpio = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(posicion) = resto.find('|') {
        limpio.push_str(&resto[..posicion]);
        let tras = &resto[posicion + 1..];
        resto = if let Some(r) = tras.strip_prefix(['r', 'R']) {
            r
        } else if let Some(color) = tras.strip_prefix(['c', 'C']).filter(|c| es_color(c)) {
            // Los 8 primeros bytes son dígitos hexadecimales ASCII: cortar ahí respeta UTF-8.
            &color[DIGITOS_COLOR..]
        } else {
            limpio.push('|');
            tras
        };
    }
    limpio.push_str(resto);
    limpio.trim().to_owned()
}

fn es_color(texto: &str) -> bool {
    texto
        .as_bytes()
        .get(..DIGITOS_COLOR)
        .is_some_and(|b| b.iter().all(u8::is_ascii_hexdigit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_los_campos_de_un_toc_medido() {
        let toc = leer(
            "## Interface: 16001\n\
             ## Title: Azeroth Pilot Reloaded\n\
             ## Version: v7.7.1\n\
             ## X-Website: https://github.com/x/y\n\
             ## SavedVariables: APRSettings, APRData\n\
             ## SavedVariablesPerCharacter: APRNavigationCharacterSettings\n\
             Core.lua\n",
        );
        assert_eq!(
            toc,
            Toc {
                titulo: Some("Azeroth Pilot Reloaded".into()),
                version: Some("v7.7.1".into()),
                enlace: Some("https://github.com/x/y".into()),
                dependencias: vec![],
                sv_cuenta: vec!["APRSettings".into(), "APRData".into()],
                sv_personaje: vec!["APRNavigationCharacterSettings".into()],
            }
        );
    }

    #[test]
    fn bom_y_crlf_no_afectan() {
        let toc = leer("\u{feff}## Title: Pawn\r\n## SavedVariables: PawnCommon\r\n");
        assert_eq!(toc.titulo.as_deref(), Some("Pawn"));
        assert_eq!(toc.sv_cuenta, ["PawnCommon"]);
    }

    #[test]
    fn el_titulo_pierde_los_colores() {
        assert_eq!(
            leer("## Title: |cff0cd29fEllesmereUI|r Action Bars").titulo,
            Some("EllesmereUI Action Bars".into())
        );
        assert_eq!(sin_colores("a || b |cZZ"), "a || b |cZZ");
        assert_eq!(leer("## Title: |cffEEE4AE|r").titulo, None);
    }

    #[test]
    fn listas_vacias_y_sin_campo_quedan_vacias() {
        let toc = leer("## SavedVariablesPerCharacter:\n## Title:\n");
        assert_eq!(toc, Toc::default());
    }

    #[test]
    fn dependencias_juntas_y_sin_repetir_y_sin_opcionales() {
        let toc = leer(
            "## Dependencies: Details, Masque\n## RequiredDeps: Masque, BlizzMove\n## OptionalDeps: Ace3\n",
        );
        assert_eq!(toc.dependencias, ["Details", "Masque", "BlizzMove"]);
    }

    #[test]
    fn claves_sin_mayusculas_y_la_primera_gana() {
        let toc = leer("##title: Uno\n## TITLE : Dos\n## AllowLoadGameType selector\n");
        assert_eq!(toc.titulo.as_deref(), Some("Uno"));
    }

    #[test]
    fn el_enlace_conserva_los_dos_puntos() {
        assert_eq!(
            leer("## X-Website: https://raider.io").enlace.as_deref(),
            Some("https://raider.io")
        );
    }
}
