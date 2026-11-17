//! Descubrimiento de addons (ver `docs/arquitectura.md`): los addons instalados de una versión del juego, según
//! `docs/modelo-de-datos.md` («Addon instalado»). Solo lee. Las carpetas sin `.toc` cargable no se listan, pero se
//! cuentan, para que un error de regla se note.

use crate::deteccion::Producto;
use crate::disco::{Subcarpeta, leer_texto, subcarpetas};
use crate::error::ErrorNucleo;
use crate::toc::{self, Toc};
use serde::Serialize;
use std::path::Path;

/// Los addons de `Interface/AddOns` de una versión.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Addons {
    pub addons: Vec<Addon>,
    /// Carpetas sin un `.toc` que el juego cargue (`…-bak` con el `.toc` de otro nombre, carpetas vacías…).
    pub ignoradas: u32,
}

/// La identidad de un addon es su carpeta, junto con la versión del juego.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Addon {
    pub carpeta: String,
    /// El `.toc` que se ha leído (`APR_Camelot.toc`, `Details.toc`…).
    pub toc: String,
    #[serde(flatten)]
    pub datos: Toc,
}

/// Descubre los addons de `carpeta_addons` (`<versión>/Interface/AddOns`). Si no existe, no hay addons.
pub fn descubrir_addons(carpeta_addons: &Path, producto: Producto) -> Result<Addons, ErrorNucleo> {
    let mut addons = Vec::new();
    let mut ignoradas = 0;
    for carpeta in subcarpetas(carpeta_addons)? {
        match leer_toc_cargable(&carpeta, producto)? {
            Some((toc, texto)) => addons.push(Addon {
                datos: toc::leer(&texto),
                carpeta: carpeta.nombre,
                toc,
            }),
            None => ignoradas += 1,
        }
    }
    Ok(Addons { addons, ignoradas })
}

/// El primer `.toc` de `nombres_de_toc` que exista, con su texto.
fn leer_toc_cargable(
    carpeta: &Subcarpeta,
    producto: Producto,
) -> Result<Option<(String, String)>, ErrorNucleo> {
    for nombre in nombres_de_toc(&carpeta.nombre, producto) {
        if let Some(texto) = leer_texto(&carpeta.ruta.join(&nombre))? {
            return Ok(Some((nombre, texto)));
        }
    }
    Ok(None)
}

/// El juego solo carga un `.toc` con el nombre de su carpeta [V]: primero el de su tipo de juego y después el
/// general. Forever es `camelot` [V]; Retail, `mainline` [I: regla de Blizzard, sin `_Mainline.toc` medido].
fn nombres_de_toc(carpeta: &str, producto: Producto) -> [String; 2] {
    let sufijo = match producto {
        Producto::Forever => "_Camelot",
        Producto::Retail => "_Mainline",
    };
    [format!("{carpeta}{sufijo}.toc"), format!("{carpeta}.toc")]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};

    fn addon(raiz: &Path, carpeta: &str, tocs: &[(&str, &str)]) {
        let ruta = raiz.join(carpeta);
        create_dir_all(&ruta).unwrap();
        for (nombre, texto) in tocs {
            write(ruta.join(nombre), texto).unwrap();
        }
    }

    fn resumen(addons: &Addons) -> Vec<(&str, &str, Option<&str>)> {
        addons
            .addons
            .iter()
            .map(|a| {
                (
                    a.carpeta.as_str(),
                    a.toc.as_str(),
                    a.datos.titulo.as_deref(),
                )
            })
            .collect()
    }

    /// Árbol con las formas medidas en la instalación de Sergio (spec 014).
    fn addons_medidos(raiz: &Path) {
        addon(
            raiz,
            "APR",
            &[
                ("APR.toc", "## Title: General"),
                ("APR_Camelot.toc", "## Title: Camelot"),
                ("APR_Mainline.toc", "## Title: Mainline"),
            ],
        );
        addon(
            raiz,
            "EllesmereUIForeverEssentials",
            &[(
                "EllesmereUIForeverEssentials_Camelot.toc",
                "## Title: Essentials",
            )],
        );
        addon(raiz, "Details", &[("Details.toc", "## Title: Details!")]);
        addon(
            raiz,
            "SpokenQuests_esES-bak",
            &[("SpokenQuests_esES.toc", "## Title: Copia")],
        );
        create_dir_all(raiz.join("Vacia")).unwrap();
        write(raiz.join("suelto.toc"), "## Title: Suelto").unwrap();
    }

    #[test]
    fn forever_prefiere_camelot_y_cuenta_las_carpetas_sin_toc_cargable() {
        let tmp = tempfile::tempdir().unwrap();
        addons_medidos(tmp.path());

        let addons = descubrir_addons(tmp.path(), Producto::Forever).unwrap();

        assert_eq!(
            resumen(&addons),
            [
                ("APR", "APR_Camelot.toc", Some("Camelot")),
                ("Details", "Details.toc", Some("Details!")),
                (
                    "EllesmereUIForeverEssentials",
                    "EllesmereUIForeverEssentials_Camelot.toc",
                    Some("Essentials")
                ),
            ]
        );
        assert_eq!(addons.ignoradas, 2);
    }

    #[test]
    fn retail_prefiere_mainline_y_no_carga_camelot() {
        let tmp = tempfile::tempdir().unwrap();
        addons_medidos(tmp.path());

        let addons = descubrir_addons(tmp.path(), Producto::Retail).unwrap();

        assert_eq!(
            resumen(&addons),
            [
                ("APR", "APR_Mainline.toc", Some("Mainline")),
                ("Details", "Details.toc", Some("Details!")),
            ]
        );
        assert_eq!(addons.ignoradas, 3);
    }

    #[test]
    fn sin_carpeta_addons_no_hay_addons() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            descubrir_addons(&tmp.path().join("no"), Producto::Forever).unwrap(),
            Addons {
                addons: vec![],
                ignoradas: 0
            }
        );
    }

    #[test]
    fn se_serializa_plano_y_en_camel_case() {
        let tmp = tempfile::tempdir().unwrap();
        addon(
            tmp.path(),
            "Pawn",
            &[(
                "Pawn.toc",
                "## Title: Pawn\n## SavedVariablesPerCharacter: PawnOptions",
            )],
        );

        let json =
            serde_json::to_value(descubrir_addons(tmp.path(), Producto::Retail).unwrap()).unwrap();

        let pawn = &json["addons"][0];
        assert_eq!(pawn["carpeta"], "Pawn");
        assert_eq!(pawn["titulo"], "Pawn");
        assert_eq!(pawn["svPersonaje"][0], "PawnOptions");
        assert!(pawn["version"].is_null());
    }
}
