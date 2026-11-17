//! Descubrimiento (ver `docs/arquitectura.md`): cuentas y personajes de una versión del juego, según
//! `docs/modelo-de-datos.md`. Solo lee. Lo que no encaja en las reglas no se lista, pero se cuenta, para que un
//! error de regla se note.

use crate::deteccion::Producto;
use crate::disco::subcarpetas;
use crate::error::ErrorNucleo;
use serde::Serialize;
use std::path::Path;

/// Carpeta de cada cuenta y de cada personaje que guarda `SavedVariables` y no es un reino.
const CARPETA_SAVED_VARIABLES: &str = "SavedVariables";

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
    pub personajes: Vec<Personaje>,
    /// Carpetas de reino de la otra versión del juego: no se listan.
    pub reinos_ignorados: u32,
}

/// La identidad de un personaje es su carpeta (cuenta, reino y personaje), nunca su nombre: el nombre de la
/// carpeta no siempre es el del personaje (`Goldber8D31D`). El nombre desde los datos llega con #37.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Personaje {
    pub carpeta_reino: String,
    pub carpeta_personaje: String,
}

/// Descubre las cuentas de `carpeta_account` (`<versión>/WTF/Account`). Si no existe, no hay cuentas.
pub fn descubrir(carpeta_account: &Path, producto: Producto) -> Result<Cuentas, ErrorNucleo> {
    let mut cuentas = Vec::new();
    let mut ignoradas = 0;
    for carpeta in subcarpetas(carpeta_account)? {
        if es_cuenta(&carpeta.nombre) {
            cuentas.push(leer_cuenta(carpeta.nombre, &carpeta.ruta, producto)?);
        } else {
            ignoradas += 1;
        }
    }
    Ok(Cuentas { cuentas, ignoradas })
}

fn leer_cuenta(carpeta: String, ruta: &Path, producto: Producto) -> Result<Cuenta, ErrorNucleo> {
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
                carpeta_reino: reino.nombre.clone(),
                carpeta_personaje: personaje.nombre,
            });
        }
    }
    Ok(Cuenta {
        carpeta,
        personajes,
        reinos_ignorados,
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

    fn personaje(reino: &str, nombre: &str) -> Personaje {
        Personaje {
            carpeta_reino: reino.into(),
            carpeta_personaje: nombre.into(),
        }
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

        let resultado = descubrir(tmp.path(), Producto::Forever).unwrap();

        assert_eq!(resultado.ignoradas, 2);
        assert_eq!(resultado.cuentas.len(), 1);
        let cuenta = &resultado.cuentas[0];
        assert_eq!(cuenta.carpeta, "1#1");
        assert_eq!(cuenta.reinos_ignorados, 1);
        assert_eq!(
            cuenta.personajes,
            [
                personaje("93", "Earthcaller-Goldberg"),
                personaje("93", "Goldberg-Joestar")
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

        let cuenta = &descubrir(tmp.path(), Producto::Retail).unwrap().cuentas[0];

        assert_eq!(cuenta.reinos_ignorados, 1);
        assert_eq!(
            cuenta.personajes,
            [
                personaje("C'Thun", "Gøldberg"),
                personaje("Sanguino", "Goldber8D31D")
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
        let json = serde_json::to_value(personaje("93", "Goldberg-Joestar")).unwrap();
        assert_eq!(json["carpetaReino"], "93");
        assert_eq!(json["carpetaPersonaje"], "Goldberg-Joestar");
    }
}
