//! Claves de personaje de los `SavedVariables` de una cuenta: las de cada tabla AceDB (`profileKeys` con `profiles`
//! al lado), según `docs/modelo-de-datos.md`. Solo lee. Los ficheros se parsean con `full_moon` sin ejecutarlos.
//!
//! D-02: un SavedVariables guarda bytes, no texto UTF-8. Se lee como Latin-1 (1 byte = 1 `char`) para que el
//! parser lo acepte entero, y cada clave se recompone a UTF-8 al final (`Gøldberg`).

use crate::disco::ficheros_con_extension;
use crate::error::ErrorNucleo;
use crate::lua::{CLAVE_PERFILES, CLAVE_PERSONAJES};
use full_moon::ast::{Expression, Field, TableConstructor};
use full_moon::tokenizer::{TokenReference, TokenType};
use full_moon::visitors::Visitor;
use std::collections::HashSet;
use std::path::Path;

const EXTENSION_LUA: &str = "lua";

/// Claves de personaje de todos los `.lua` de `carpeta_sv` (`<cuenta>/SavedVariables`).
pub fn claves_de_personaje(carpeta_sv: &Path) -> Result<HashSet<String>, ErrorNucleo> {
    let mut claves = HashSet::new();
    for ruta in ficheros_con_extension(carpeta_sv, EXTENSION_LUA)? {
        let bytes = std::fs::read(&ruta).map_err(|e| ErrorNucleo::lectura(&ruta, &e))?;
        claves.extend(claves_de_lua(&bytes));
    }
    Ok(claves)
}

/// Claves de personaje de un fichero. Sin `profileKeys` no se parsea: de 215 ficheros medidos, solo 50 lo traen.
/// Un fichero que no parsea no aporta claves [D-02 midió 0 de 1.467]: el personaje cae a la regla de la carpeta,
/// que no inventa nada.
fn claves_de_lua(bytes: &[u8]) -> Vec<String> {
    let marca = CLAVE_PERSONAJES.as_bytes();
    if !bytes.windows(marca.len()).any(|ventana| ventana == marca) {
        return Vec::new();
    }
    let latin1: String = bytes.iter().map(|&b| char::from(b)).collect();
    let Ok(ast) = full_moon::parse(&latin1) else {
        return Vec::new();
    };
    let mut buscador = BuscadorAceDb::default();
    buscador.visit_ast(&ast);
    buscador.claves
}

/// Recorre todas las tablas, a cualquier profundidad, y guarda las claves de las `profileKeys` de AceDB.
#[derive(Default)]
struct BuscadorAceDb {
    claves: Vec<String>,
}

impl Visitor for BuscadorAceDb {
    fn visit_table_constructor(&mut self, tabla: &TableConstructor) {
        let mut personajes = None;
        let mut tiene_perfiles = false;
        for campo in tabla.fields() {
            match (clave_de(campo).as_deref(), valor_de(campo)) {
                (Some(CLAVE_PERSONAJES), Some(Expression::TableConstructor(t))) => {
                    personajes = Some(t)
                }
                (Some(CLAVE_PERFILES), _) => tiene_perfiles = true,
                _ => {}
            }
        }
        if let (Some(personajes), true) = (personajes, tiene_perfiles) {
            let claves = personajes.fields().iter().filter_map(clave_de);
            self.claves.extend(claves.filter_map(|c| a_utf8(&c)));
        }
    }
}

/// La clave de un campo `["texto"] = …` o `nombre = …`, todavía en Latin-1. Las claves numéricas no cuentan.
fn clave_de(campo: &Field) -> Option<String> {
    match campo {
        Field::ExpressionKey {
            key: Expression::String(token),
            ..
        } => literal(token),
        Field::NameKey { key, .. } => Some(key.token().to_string()),
        _ => None,
    }
}

fn valor_de(campo: &Field) -> Option<&Expression> {
    match campo {
        Field::ExpressionKey { value, .. } | Field::NameKey { value, .. } => Some(value),
        _ => None,
    }
}

/// El texto de una cadena, sin comillas. Los escapes (`\"`) llegan sin procesar [I: ninguno medido en claves].
fn literal(token: &TokenReference) -> Option<String> {
    match token.token_type() {
        TokenType::StringLiteral { literal, .. } => Some(literal.to_string()),
        _ => None,
    }
}

/// Deshace la lectura en Latin-1: cada `char` vuelve a ser su byte, y los bytes se leen como UTF-8.
fn a_utf8(latin1: &str) -> Option<String> {
    let bytes = latin1
        .chars()
        .map(|c| u8::try_from(u32::from(c)).ok())
        .collect::<Option<Vec<u8>>>()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordenadas(bytes: &[u8]) -> Vec<String> {
        let mut claves = claves_de_lua(bytes);
        claves.sort();
        claves
    }

    #[test]
    fn lee_las_claves_de_una_tabla_acedb_anidada() {
        let lua = "PlaterDB = {\n\
                   \t[\"profileKeys\"] = {\n\
                   \t\t[\"Goldberg Joestar\"] = \"Default\",\n\
                   \t\t[\"Goldberg - C'Thun\"] = \"Goldberg - C'Thun\",\n\
                   \t},\n\
                   \t[\"profiles\"] = { [\"Default\"] = {} },\n\
                   }\n\
                   Otro = { db = { profileKeys = { Shade = \"Default\" }, profiles = {} } }\n";
        assert_eq!(
            ordenadas(lua.as_bytes()),
            ["Goldberg - C'Thun", "Goldberg Joestar", "Shade"]
        );
    }

    #[test]
    fn sin_profiles_al_lado_no_es_acedb() {
        assert!(ordenadas(b"X = { profileKeys = { [\"Goldberg\"] = 1 } }").is_empty());
    }

    #[test]
    fn la_o_barrada_se_recompone_a_utf8() {
        let lua =
            "X = { profiles = {}, profileKeys = { [\"Gøldberg - Sanguino\"] = \"Default\" } }";
        assert_eq!(ordenadas(lua.as_bytes()), ["Gøldberg - Sanguino"]);
    }

    #[test]
    fn un_fichero_que_no_parsea_no_aporta_claves() {
        assert!(ordenadas(b"X = { profiles = {}, profileKeys = { [\"A\"] = ").is_empty());
    }

    #[test]
    fn bytes_que_no_son_utf8_no_rompen_la_lectura() {
        let mut lua = b"X = { profiles = {}, profileKeys = { [\"Bien\"] = 1, [\"".to_vec();
        lua.extend([
            0xff, b'"', b']', b' ', b'=', b' ', b'2', b' ', b'}', b' ', b'}',
        ]);
        assert_eq!(ordenadas(&lua), ["Bien"]);
    }

    #[test]
    fn lee_solo_los_lua_de_la_carpeta() {
        let tmp = tempfile::tempdir().unwrap();
        let acedb = "X = { profiles = {}, profileKeys = { [\"Goldberg Sp\"] = 1 } }";
        std::fs::write(tmp.path().join("Plater.lua"), acedb).unwrap();
        std::fs::write(
            tmp.path().join("Plater.lua.bak"),
            acedb.replace("Sp", "Bak"),
        )
        .unwrap();
        std::fs::write(tmp.path().join("Vacio.lua"), "Y = {}").unwrap();

        let claves = claves_de_personaje(tmp.path()).unwrap();

        assert_eq!(claves, HashSet::from(["Goldberg Sp".to_owned()]));
        assert!(
            claves_de_personaje(&tmp.path().join("no"))
                .unwrap()
                .is_empty()
        );
    }
}
