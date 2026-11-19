//! Descubrimiento (ver `docs/architecture.md`): cuentas y personajes de una versión del juego, según
//! `docs/data-model.md`. Solo lee. Lo que no encaja en las reglas no se lista, pero se cuenta, para que un
//! error de regla se note.

use crate::claves_personaje::claves_de_personaje;
use crate::deteccion::Producto;
use crate::disco::subcarpetas;
use crate::error::ErrorNucleo;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

/// Carpeta de cada cuenta y de cada personaje que guarda `SavedVariables` y no es un reino.
pub const CARPETA_SAVED_VARIABLES: &str = "SavedVariables";

/// Las cuentas de `WTF/Account` de una versión.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cuentas {
    pub cuentas: Vec<Cuenta>,
    /// Carpetas de `WTF/Account` que no tienen forma de cuenta (`…#1.back`, `SavedVariables`…).
    pub ignoradas: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cuenta {
    pub carpeta: String,
    /// Nombre propio que le ha puesto el usuario (#34). Lo rellena Detección con las preferencias.
    pub nombre: Option<String>,
    pub personajes: Vec<Personaje>,
    /// Carpetas de reino de la otra versión del juego: no se listan.
    pub reinos_ignorados: u32,
}

/// La identidad de un personaje es su carpeta (cuenta, reino y personaje), nunca su nombre: el nombre de la
/// carpeta no siempre es el del personaje (`Goldber8D31D`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Personaje {
    pub carpeta_reino: String,
    pub carpeta_personaje: String,
    /// Cómo se llama en los `SavedVariables`. `None`: sin resolver; se preguntará, nunca se inventa.
    pub nombre_en_datos: Option<NombreResuelto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NombreResuelto {
    pub texto: String,
    pub origen: OrigenNombre,
}

/// `Datos`: la clave existe en los `SavedVariables` de la cuenta. `Carpeta`: deducido de la carpeta, sin confirmar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OrigenNombre {
    Datos,
    Carpeta,
}

/// Descubre las cuentas de `carpeta_account` (`<versión>/WTF/Account`). Si no existe, no hay cuentas.
pub fn descubrir(carpeta_account: &Path, producto: Producto) -> Result<Cuentas, ErrorNucleo> {
    let mut cuentas = Vec::new();
    let mut ignoradas = 0;
    for carpeta in subcarpetas(carpeta_account)? {
        if es_cuenta(&carpeta.nombre) {
            cuentas.push(leer_cuenta(carpeta.nombre, &carpeta.ruta, producto)?);
        } else {
            log::info!(
                "{}: saltada, no tiene la forma de una cuenta (<número>#<número>)",
                carpeta.ruta.display()
            );
            ignoradas += 1;
        }
    }
    Ok(Cuentas { cuentas, ignoradas })
}

fn leer_cuenta(carpeta: String, ruta: &Path, producto: Producto) -> Result<Cuenta, ErrorNucleo> {
    let claves = claves_de_personaje(&ruta.join(CARPETA_SAVED_VARIABLES))?;
    let mut personajes = Vec::new();
    let mut reinos_ignorados = 0;
    for reino in subcarpetas(ruta)? {
        if reino.nombre == CARPETA_SAVED_VARIABLES {
            continue;
        }
        if !es_reino_de(&reino.nombre, producto) {
            reinos_ignorados += 1;
            continue;
        }
        for personaje in subcarpetas(&reino.ruta)? {
            personajes.push(Personaje {
                nombre_en_datos: resolver_nombre(
                    &reino.nombre,
                    &personaje.nombre,
                    producto,
                    &claves,
                ),
                carpeta_reino: reino.nombre.clone(),
                carpeta_personaje: personaje.nombre,
            });
        }
    }
    Ok(Cuenta {
        carpeta,
        nombre: None,
        personajes,
        reinos_ignorados,
    })
}

/// Cómo se llamaría en los datos cada personaje de la cuenta `ruta_cuenta`, según su carpeta (#56: saber si un
/// fichero de la cuenta menciona a sus personajes).
pub fn nombres_de_carpetas(
    ruta_cuenta: &Path,
    producto: Producto,
) -> Result<Vec<String>, ErrorNucleo> {
    let mut nombres = Vec::new();
    for reino in subcarpetas(ruta_cuenta)? {
        if reino.nombre == CARPETA_SAVED_VARIABLES || !es_reino_de(&reino.nombre, producto) {
            continue;
        }
        for personaje in subcarpetas(&reino.ruta)? {
            nombres.extend(candidato(&reino.nombre, &personaje.nombre, producto));
        }
    }
    Ok(nombres)
}

/// Orden de `docs/data-model.md`: el candidato de la carpeta, confirmado si es una clave de los datos.
fn resolver_nombre(
    carpeta_reino: &str,
    carpeta_personaje: &str,
    producto: Producto,
    claves: &HashSet<String>,
) -> Option<NombreResuelto> {
    let texto = candidato(carpeta_reino, carpeta_personaje, producto)?;
    let origen = if claves.contains(&texto) {
        OrigenNombre::Datos
    } else {
        OrigenNombre::Carpeta
    };
    Some(NombreResuelto { texto, origen })
}

/// Cómo se llamaría en los datos según su carpeta: Forever `Nombre-Apellido` → `"Nombre Apellido"` [V], Retail
/// `"Nombre - Reino"` [V]. Una carpeta con cifras lleva el final del GUID (`Goldber8D31D`) [V] y los nombres de WoW
/// no llevan cifras [I]: de ella no sale ningún nombre.
fn candidato(carpeta_reino: &str, carpeta_personaje: &str, producto: Producto) -> Option<String> {
    if carpeta_personaje.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(match producto {
        Producto::Forever => carpeta_personaje.replace('-', " "),
        Producto::Retail => format!("{carpeta_personaje} - {carpeta_reino}"),
    })
}

/// Una cuenta tiene forma `<número>#<número>` [V].
fn es_cuenta(nombre: &str) -> bool {
    nombre
        .split_once('#')
        .is_some_and(|(id, indice)| solo_digitos(id) && solo_digitos(indice))
}

/// En Forever los reinos son carpetas numéricas (`93`) [H como regla general]; en Retail, el nombre del reino [V].
/// Una carpeta de la otra forma es un reino copiado de la otra versión.
fn es_reino_de(nombre: &str, producto: Producto) -> bool {
    match producto {
        Producto::Forever => solo_digitos(nombre),
        Producto::Retail => !solo_digitos(nombre),
    }
}

fn solo_digitos(texto: &str) -> bool {
    !texto.is_empty() && texto.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::create_dir_all;

    fn crear(raiz: &Path, rutas: &[&str]) {
        for ruta in rutas {
            create_dir_all(raiz.join(ruta)).unwrap();
        }
    }

    fn personaje(reino: &str, carpeta: &str, nombre: Option<(&str, OrigenNombre)>) -> Personaje {
        Personaje {
            carpeta_reino: reino.into(),
            carpeta_personaje: carpeta.into(),
            nombre_en_datos: nombre.map(|(texto, origen)| NombreResuelto {
                texto: texto.into(),
                origen,
            }),
        }
    }

    fn acedb(raiz: &Path, cuenta: &str, claves: &[&str]) {
        let campos: String = claves
            .iter()
            .map(|c| format!("[\"{c}\"] = \"Default\", "))
            .collect();
        let carpeta = raiz.join(cuenta).join(CARPETA_SAVED_VARIABLES);
        create_dir_all(&carpeta).unwrap();
        std::fs::write(
            carpeta.join("Plater.lua"),
            format!("PlaterDB = {{ profileKeys = {{ {campos} }}, profiles = {{}} }}"),
        )
        .unwrap();
    }

    #[test]
    fn forever_lista_personajes_de_reinos_numericos_y_cuenta_el_resto() {
        let tmp = tempfile::tempdir().unwrap();
        crear(
            tmp.path(),
            &[
                "1#1/93/Goldberg-Joestar",
                "1#1/93/Earthcaller-Goldberg",
                "1#1/93/Goldberg-Joestar/SavedVariables",
                "1#1/SavedVariables",
                "1#1/C'Thun/Goldberg",
                "1#1.back/93/Otro",
                "SavedVariables",
            ],
        );
        std::fs::write(tmp.path().join("1#1/macros-cache.txt"), "").unwrap();
        acedb(
            tmp.path(),
            "1#1",
            &["Goldberg Joestar", "Maxander - Dun Modr"],
        );

        let resultado = descubrir(tmp.path(), Producto::Forever).unwrap();

        assert_eq!(resultado.ignoradas, 2);
        assert_eq!(resultado.cuentas.len(), 1);
        let cuenta = &resultado.cuentas[0];
        assert_eq!(cuenta.carpeta, "1#1");
        assert_eq!(cuenta.reinos_ignorados, 1);
        assert_eq!(
            cuenta.personajes,
            [
                personaje(
                    "93",
                    "Earthcaller-Goldberg",
                    Some(("Earthcaller Goldberg", OrigenNombre::Carpeta))
                ),
                personaje(
                    "93",
                    "Goldberg-Joestar",
                    Some(("Goldberg Joestar", OrigenNombre::Datos))
                )
            ]
        );
    }

    #[test]
    fn retail_lista_reinos_con_nombre_e_ignora_los_numericos() {
        let tmp = tempfile::tempdir().unwrap();
        crear(
            tmp.path(),
            &[
                "2#3/C'Thun/Gøldberg",
                "2#3/Sanguino/Goldber8D31D",
                "2#3/93/Goldberg-Joestar",
            ],
        );
        acedb(
            tmp.path(),
            "2#3",
            &["Gøldberg - C'Thun", "Gøldberg - Sanguino"],
        );

        let cuenta = &descubrir(tmp.path(), Producto::Retail).unwrap().cuentas[0];

        assert_eq!(cuenta.reinos_ignorados, 1);
        assert_eq!(
            cuenta.personajes,
            [
                personaje(
                    "C'Thun",
                    "Gøldberg",
                    Some(("Gøldberg - C'Thun", OrigenNombre::Datos))
                ),
                personaje("Sanguino", "Goldber8D31D", None)
            ]
        );
    }

    #[test]
    fn sin_carpeta_account_no_hay_cuentas() {
        let tmp = tempfile::tempdir().unwrap();
        let resultado = descubrir(&tmp.path().join("no"), Producto::Forever).unwrap();
        assert_eq!(
            resultado,
            Cuentas {
                cuentas: vec![],
                ignoradas: 0
            }
        );
    }

    #[test]
    fn forma_de_cuenta() {
        assert!(es_cuenta("107710745#1"));
        for nombre in ["107710745#1.back", "SavedVariables", "#1", "1#", "1#a", "1"] {
            assert!(!es_cuenta(nombre), "{nombre}");
        }
    }

    #[test]
    fn se_serializa_en_camel_case() {
        let json = serde_json::to_value(personaje(
            "93",
            "Goldberg-Joestar",
            Some(("Goldberg Joestar", OrigenNombre::Datos)),
        ))
        .unwrap();
        assert_eq!(json["carpetaReino"], "93");
        assert_eq!(json["carpetaPersonaje"], "Goldberg-Joestar");
        assert_eq!(json["nombreEnDatos"]["texto"], "Goldberg Joestar");
        assert_eq!(json["nombreEnDatos"]["origen"], "datos");
        let sin = serde_json::to_value(personaje("Sanguino", "Bison2D457", None)).unwrap();
        assert!(sin["nombreEnDatos"].is_null());
    }
}
