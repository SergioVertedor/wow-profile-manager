//! Catálogo de adaptadores (`docs/adaptadores.md`): un JSON por addon en `src-tauri/adaptadores/`, dentro del
//! ejecutable. Son las excepciones medidas a las reglas genéricas (spec 027).

use crate::paquetes::Ambito;
use serde::Deserialize;
use std::sync::OnceLock;

/// Los ficheros del catálogo. Uno nuevo se añade aquí y en `src-tauri/adaptadores/`.
const FICHEROS: [&str; 11] = [
    include_str!("../adaptadores/auctionator.json"),
    include_str!("../adaptadores/buggrabber.json"),
    include_str!("../adaptadores/ellesmereui.json"),
    include_str!("../adaptadores/forever-dungeon-journal.json"),
    include_str!("../adaptadores/layerswap.json"),
    include_str!("../adaptadores/lfg-forever.json"),
    include_str!("../adaptadores/pawn.json"),
    include_str!("../adaptadores/skillup-forever.json"),
    include_str!("../adaptadores/spy.json"),
    include_str!("../adaptadores/talents-forever-book.json"),
    include_str!("../adaptadores/wim.json"),
];

/// Un adaptador (`docs/adaptador.schema.json`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Adaptador {
    pub id: String,
    pub version: u32,
    pub addon: String,
    #[serde(default)]
    pub versiones_addon: Vec<String>,
    /// Solo documenta el adaptador; se lee para que `deny_unknown_fields` lo exija.
    #[expect(
        dead_code,
        reason = "documentación del adaptador, no se usa al exportar"
    )]
    medido: String,
    #[serde(default)]
    pub reglas: Vec<Regla>,
    #[serde(default)]
    pub no_exportar: bool,
}

/// Qué viaja de una variable. Una sola forma de seleccionar por regla.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Regla {
    pub ambito: Ambito,
    pub variable: String,
    #[serde(default)]
    pub entera: bool,
    pub incluir: Option<Vec<String>>,
    #[serde(default)]
    pub excluir: Vec<String>,
    #[serde(default)]
    pub perfil_acedb: bool,
    #[serde(default)]
    pub solo_personaje: Vec<String>,
    #[serde(default)]
    pub clave_personaje: ClavePersonaje,
}

impl Regla {
    /// `entera` y `excluir` dejan pasar claves que nadie ha listado: solo valen con la versión medida.
    pub fn deja_pasar_sin_nombrar(&self) -> bool {
        self.entera || !self.excluir.is_empty()
    }
}

/// Cómo escribe el addon la clave del personaje.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClavePersonaje {
    /// `"Nombre Apellido"`, como AceDB en Forever.
    #[default]
    NombreApellido,
    /// `"Nombre-Reino"`: ningún adaptador medido la usa; no encuentra personaje.
    NombreGuionReino,
}

impl Adaptador {
    /// Las reglas que dejan pasar claves que nadie ha listado (`entera`, `excluir`) solo valen con la versión medida.
    pub fn version_medida(&self, instalada: Option<&str>) -> bool {
        let necesita = self.reglas.iter().any(Regla::deja_pasar_sin_nombrar);
        !necesita || instalada.is_some_and(|v| self.versiones_addon.iter().any(|m| m == v))
    }
}

/// El catálogo, leído una vez. Un fichero que no se entiende no entra (lo vigila un test).
pub fn catalogo() -> &'static [Adaptador] {
    static CATALOGO: OnceLock<Vec<Adaptador>> = OnceLock::new();
    CATALOGO.get_or_init(|| {
        FICHEROS
            .iter()
            .filter_map(|json| serde_json::from_str(json).ok())
            .collect()
    })
}

/// El adaptador de un addon (por su carpeta), si lo hay.
pub fn de_addon(carpeta: &str) -> Option<&'static Adaptador> {
    catalogo()
        .iter()
        .find(|a| a.addon.eq_ignore_ascii_case(carpeta))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn todo_el_catalogo_se_entiende() {
        for json in FICHEROS {
            if let Err(e) = serde_json::from_str::<Adaptador>(json) {
                panic!("{e}: {json}");
            }
        }
        assert_eq!(catalogo().len(), FICHEROS.len());
    }

    #[test]
    fn ids_y_addons_unicos() {
        let ids: HashSet<_> = catalogo().iter().map(|a| &a.id).collect();
        let addons: HashSet<_> = catalogo().iter().map(|a| a.addon.to_lowercase()).collect();
        assert_eq!(ids.len(), FICHEROS.len());
        assert_eq!(addons.len(), FICHEROS.len());
    }

    #[test]
    fn cada_regla_elige_una_forma_y_lo_abierto_exige_version() {
        for adaptador in catalogo() {
            assert_ne!(
                adaptador.no_exportar,
                !adaptador.reglas.is_empty(),
                "{}",
                adaptador.id
            );
            for regla in &adaptador.reglas {
                let formas = [
                    regla.entera,
                    regla.incluir.is_some(),
                    !regla.excluir.is_empty(),
                    regla.perfil_acedb,
                ];
                assert_eq!(formas.iter().filter(|f| **f).count(), 1, "{}", adaptador.id);
                if regla.perfil_acedb {
                    assert_eq!(regla.ambito, Ambito::Cuenta, "{}", adaptador.id);
                }
                // `soloPersonaje` sin `$PJ` dejaría pasar las entradas de todos los personajes.
                for ruta in &regla.solo_personaje {
                    assert!(
                        ruta.split('/').any(|s| s == "$PJ"),
                        "{}: {ruta}",
                        adaptador.id
                    );
                }
                if regla.entera || !regla.excluir.is_empty() {
                    assert!(!adaptador.versiones_addon.is_empty(), "{}", adaptador.id);
                }
            }
        }
    }

    #[test]
    fn version_medida_solo_importa_si_hay_algo_abierto() {
        let wim = de_addon("WIM").unwrap();
        assert!(wim.version_medida(Some("3.19.7")));
        assert!(!wim.version_medida(Some("3.20.0")));
        assert!(!wim.version_medida(None));
        let pawn = de_addon("pawn").unwrap();
        assert!(pawn.version_medida(None));
    }
}
