//! Detección (ver `docs/arquitectura.md`): encuentra las instalaciones de WoW y sus versiones del juego, según
//! `docs/modelo-de-datos.md`. Solo lee. La fuente de este equipo es el registro de Windows; la carpeta elegida a
//! mano llega con #34 y el agente de Battle.net con #35.

use crate::addons::{Addons, descubrir_addons};
use crate::bpsv;
use crate::descubrimiento::{Cuentas, descubrir};
use crate::disco::{leer_texto, subcarpetas};
use crate::error::ErrorNucleo;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// De dónde salió una instalación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Fuente {
    Registro,
}

/// Productos con los que trabaja la app. Los demás se listan como no soportados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Producto {
    Retail,
    Forever,
}

impl Producto {
    /// Según el valor de `.flavor.info`: `wow` en `_retail_`, `wowf` en `_forever_` [V].
    fn desde_flavor(flavor: &str) -> Option<Self> {
        match flavor {
            "wow" => Some(Self::Retail),
            "wowf" => Some(Self::Forever),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instalacion {
    /// Ruta absoluta de la raíz. Nunca sale del equipo: no va al paquete.
    pub raiz: PathBuf,
    pub fuente: Fuente,
    pub versiones: Vec<VersionJuego>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJuego {
    pub carpeta: String,
    /// Valor de `.flavor.info` tal cual (`wow`, `wowf`…).
    pub producto: String,
    pub soportada: bool,
    /// Columna `Version` de `.build.info` en la fila del producto.
    pub build: Option<String>,
    /// `engineSurveyPatch` de `WTF/Config.wtf`. Sin él, la versión no se puede exportar todavía.
    pub interfaz: Option<String>,
    /// Vacío si la versión no está soportada: no se mira dentro.
    pub cuentas: Cuentas,
    /// Vacío si la versión no está soportada.
    pub addons: Addons,
}

const BUILD_INFO: &str = ".build.info";
const FLAVOR_INFO: &str = ".flavor.info";
const COLUMNA_FLAVOR: &str = "Product Flavor";
const CARPETA_WTF: &str = "WTF";
const CARPETA_ACCOUNT: &str = "Account";
const CONFIG_WTF: &str = "Config.wtf";
const CARPETA_INTERFACE: &str = "Interface";
const CARPETA_ADDONS: &str = "AddOns";
const PREFIJO_INTERFAZ: &str = "SET engineSurveyPatch ";

/// Instalaciones de este equipo, buscadas en todas las fuentes disponibles.
pub fn detectar_en_el_sistema() -> Result<Vec<Instalacion>, ErrorNucleo> {
    let candidatas: Vec<_> = rutas_del_registro()
        .into_iter()
        .map(|ruta| (ruta, Fuente::Registro))
        .collect();
    detectar(&candidatas)
}

/// Convierte raíces candidatas en instalaciones. Una carpeta solo es una instalación si tiene `.build.info` [V];
/// las rutas repetidas cuentan una vez (la primera fuente gana).
pub fn detectar(candidatas: &[(PathBuf, Fuente)]) -> Result<Vec<Instalacion>, ErrorNucleo> {
    let mut vistas = HashSet::new();
    let mut instalaciones = Vec::new();
    for (raiz, fuente) in candidatas {
        if !vistas.insert(clave_de_ruta(raiz)) {
            continue;
        }
        let Some(build_info) = leer_texto(&raiz.join(BUILD_INFO))? else {
            continue;
        };
        instalaciones.push(Instalacion {
            raiz: raiz.clone(),
            fuente: *fuente,
            versiones: leer_versiones(raiz, &build_info)?,
        });
    }
    Ok(instalaciones)
}

/// Windows no distingue mayúsculas en las rutas y acepta un separador final: `C:\WoW\` y `c:\wow` son la misma.
fn clave_de_ruta(ruta: &Path) -> String {
    ruta.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_lowercase()
}

/// Las versiones son las subcarpetas con `.flavor.info` (`_retail_`, `_forever_`…).
fn leer_versiones(raiz: &Path, build_info: &str) -> Result<Vec<VersionJuego>, ErrorNucleo> {
    let mut versiones = Vec::new();
    for carpeta in subcarpetas(raiz)? {
        let Some(flavor_info) = leer_texto(&carpeta.ruta.join(FLAVOR_INFO))? else {
            continue;
        };
        let producto = valor_de(&flavor_info, COLUMNA_FLAVOR).unwrap_or_default();
        versiones.push(leer_version(
            carpeta.nombre,
            &carpeta.ruta,
            producto,
            build_info,
        )?);
    }
    Ok(versiones)
}

fn leer_version(
    carpeta: String,
    ruta: &Path,
    producto: String,
    build_info: &str,
) -> Result<VersionJuego, ErrorNucleo> {
    let wtf = ruta.join(CARPETA_WTF);
    let soportado = Producto::desde_flavor(&producto);
    // De una versión no soportada solo se dice que existe: no se mira dentro.
    let (cuentas, addons, interfaz) = match soportado {
        Some(p) => (
            descubrir(&wtf.join(CARPETA_ACCOUNT), p)?,
            descubrir_addons(&ruta.join(CARPETA_INTERFACE).join(CARPETA_ADDONS), p)?,
            leer_texto(&wtf.join(CONFIG_WTF))?.and_then(|t| interfaz_de(&t)),
        ),
        None => (
            Cuentas {
                cuentas: Vec::new(),
                ignoradas: 0,
            },
            Addons {
                addons: Vec::new(),
                ignoradas: 0,
            },
            None,
        ),
    };
    Ok(VersionJuego {
        build: build_de(build_info, &producto),
        interfaz,
        soportada: soportado.is_some(),
        carpeta,
        producto,
        cuentas,
        addons,
    })
}

/// Primer valor de una columna en un fichero BPSV.
fn valor_de(texto: &str, columna: &str) -> Option<String> {
    bpsv::leer(texto)
        .first()
        .and_then(|fila| fila.get(columna))
        .map(|v| v.trim().to_owned())
}

/// Columna `Version` de la fila de `.build.info` cuyo `Product` es el de la versión [V].
fn build_de(build_info: &str, producto: &str) -> Option<String> {
    bpsv::leer(build_info)
        .into_iter()
        .find(|fila| fila.get("Product") == Some(&producto))
        .and_then(|fila| fila.get("Version").map(|v| (*v).to_owned()))
}

/// Valor de `SET engineSurveyPatch "16001"` en `Config.wtf` [V].
fn interfaz_de(config_wtf: &str) -> Option<String> {
    config_wtf.lines().find_map(|linea| {
        linea
            .trim()
            .strip_prefix(PREFIJO_INTERFAZ)
            .map(|valor| valor.trim().trim_matches('"').to_owned())
            .filter(|valor| !valor.is_empty())
    })
}

/// Raíces que dice el registro: `Uninstall\World of Warcraft`, valor `InstallLocation` [V, en la vista de 32 bits
/// (`WOW6432Node`)]. Se mira también la de 64 bits; lo repetido lo junta `detectar`.
#[cfg(windows)]
fn rutas_del_registro() -> Vec<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};

    const CLAVE: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\World of Warcraft";
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    [KEY_WOW64_32KEY, KEY_WOW64_64KEY]
        .into_iter()
        .filter_map(|vista| hklm.open_subkey_with_flags(CLAVE, KEY_READ | vista).ok())
        .filter_map(|clave| clave.get_value::<String, _>("InstallLocation").ok())
        .map(PathBuf::from)
        .collect()
}

#[cfg(not(windows))]
fn rutas_del_registro() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};

    const BUILD_INFO_MEDIDO: &str = "Branch!STRING:0|Active!DEC:1|Version!STRING:0|Product!STRING:0\n\
                                     eu|1|12.1.5.70783|wow\n\
                                     eu|1|1.60.1.70938|wowf\n";

    /// Una instalación como la medida: `_forever_` y `_retail_`, más una carpeta sin `.flavor.info`.
    fn instalacion_de_prueba(raiz: &Path) {
        write(raiz.join(BUILD_INFO), BUILD_INFO_MEDIDO).unwrap();
        for (carpeta, flavor, interfaz) in [
            ("_forever_", "wowf", "16001"),
            ("_retail_", "wow", "120105"),
        ] {
            let wtf = raiz.join(carpeta).join("WTF");
            create_dir_all(wtf.join("Account/1#1")).unwrap();
            let details = raiz.join(carpeta).join("Interface/AddOns/Details");
            create_dir_all(&details).unwrap();
            write(details.join("Details.toc"), "## Title: Details!").unwrap();
            write(
                raiz.join(carpeta).join(FLAVOR_INFO),
                format!("Product Flavor!STRING:0\r\n{flavor}\r\n"),
            )
            .unwrap();
            write(
                wtf.join("Config.wtf"),
                format!("SET locale \"esES\"\r\nSET engineSurveyPatch \"{interfaz}\"\r\n"),
            )
            .unwrap();
        }
        create_dir_all(raiz.join("Data")).unwrap();
    }

    #[test]
    fn detecta_las_versiones_con_su_build_e_interfaz() {
        let tmp = tempfile::tempdir().unwrap();
        instalacion_de_prueba(tmp.path());

        let instalaciones = detectar(&[(tmp.path().to_path_buf(), Fuente::Registro)]).unwrap();

        assert_eq!(instalaciones.len(), 1);
        let versiones = &instalaciones[0].versiones;
        let resumen: Vec<_> = versiones
            .iter()
            .map(|v| {
                (
                    v.carpeta.as_str(),
                    v.producto.as_str(),
                    v.build.as_deref(),
                    v.interfaz.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            resumen,
            [
                ("_forever_", "wowf", Some("1.60.1.70938"), Some("16001")),
                ("_retail_", "wow", Some("12.1.5.70783"), Some("120105")),
            ]
        );
        assert!(
            versiones
                .iter()
                .all(|v| v.soportada && v.cuentas.cuentas.len() == 1 && v.addons.addons.len() == 1)
        );
    }

    #[test]
    fn sin_build_info_no_es_una_instalacion() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(
            detectar(&[(tmp.path().to_path_buf(), Fuente::Registro)])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn la_misma_raiz_escrita_de_otra_forma_cuenta_una_vez() {
        let tmp = tempfile::tempdir().unwrap();
        instalacion_de_prueba(tmp.path());
        let otra_forma = PathBuf::from(format!(
            "{}\\",
            tmp.path().display().to_string().to_uppercase()
        ));

        let candidatas = [
            (tmp.path().to_path_buf(), Fuente::Registro),
            (otra_forma, Fuente::Registro),
        ];

        assert_eq!(detectar(&candidatas).unwrap().len(), 1);
    }

    #[test]
    fn un_producto_desconocido_se_lista_sin_mirar_dentro() {
        let tmp = tempfile::tempdir().unwrap();
        instalacion_de_prueba(tmp.path());
        create_dir_all(tmp.path().join("_classic_/WTF/Account/1#1")).unwrap();
        create_dir_all(tmp.path().join("_classic_/Interface/AddOns/Details")).unwrap();
        write(
            tmp.path().join("_classic_/WTF/Config.wtf"),
            "SET engineSurveyPatch \"11507\"",
        )
        .unwrap();
        write(
            tmp.path().join("_classic_/.flavor.info"),
            "Product Flavor!STRING:0\nwow_classic\n",
        )
        .unwrap();

        let instalacion = &detectar(&[(tmp.path().to_path_buf(), Fuente::Registro)]).unwrap()[0];

        let clasica = instalacion
            .versiones
            .iter()
            .find(|v| v.carpeta == "_classic_")
            .unwrap();
        assert_eq!(clasica.producto, "wow_classic");
        assert!(!clasica.soportada);
        assert!(clasica.cuentas.cuentas.is_empty());
        assert!(clasica.addons.addons.is_empty());
        assert_eq!(clasica.interfaz, None);
    }

    #[test]
    fn interfaz_solo_si_config_wtf_la_trae() {
        assert_eq!(
            interfaz_de("SET engineSurveyPatch \"16001\""),
            Some("16001".into())
        );
        assert_eq!(interfaz_de("SET locale \"esES\""), None);
        assert_eq!(interfaz_de("SET engineSurveyPatch \"\""), None);
    }

    #[test]
    fn build_de_un_producto_que_no_esta_es_none() {
        assert_eq!(build_de(BUILD_INFO_MEDIDO, "wow_classic"), None);
    }

    #[test]
    fn se_serializa_como_lo_lee_la_interfaz() {
        let tmp = tempfile::tempdir().unwrap();
        instalacion_de_prueba(tmp.path());
        let json = serde_json::to_value(
            detectar(&[(tmp.path().to_path_buf(), Fuente::Registro)]).unwrap(),
        )
        .unwrap();
        assert_eq!(json[0]["fuente"], "registro");
        assert_eq!(
            json[0]["versiones"][0]["cuentas"]["cuentas"][0]["reinosIgnorados"],
            0
        );
        assert_eq!(
            json[0]["versiones"][0]["addons"]["addons"][0]["titulo"],
            "Details!"
        );
        assert!(json[0]["raiz"].is_string());
    }
}
