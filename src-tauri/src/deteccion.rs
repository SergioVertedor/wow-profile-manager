//! Detección (ver `docs/arquitectura.md`): encuentra las instalaciones de WoW y sus versiones del juego, según
//! `docs/modelo-de-datos.md`. Solo lee. Fuentes: el registro de Windows y las carpetas elegidas a mano (spec 018,
//! guardadas en Preferencias); el agente de Battle.net llega con #35.

use crate::addons::{Addons, descubrir_addons};
use crate::agente_battlenet;
use crate::bpsv;
use crate::descubrimiento::{Cuentas, descubrir};
use crate::disco::{clave_de_ruta, leer_texto, subcarpetas};
use crate::error::ErrorNucleo;
use crate::preferencias::Preferencias;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// De dónde salió una instalación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Fuente {
    Registro,
    /// `product.db` del agente de Battle.net (spec 020). Es una pista: lo que no se puede leer se calla.
    Battlenet,
    Manual,
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
    /// Nombre propio que le ha puesto el usuario (spec 018).
    pub nombre: Option<String>,
    pub versiones: Vec<VersionJuego>,
}

/// Lo que ve la interfaz al detectar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deteccion {
    pub instalaciones: Vec<Instalacion>,
    /// Carpetas añadidas a mano que ya no son una instalación (movida, borrada, unidad desconectada). Se siguen
    /// enseñando, con su nombre, hasta que el usuario las quita.
    pub no_encontradas: Vec<CarpetaPerdida>,
    /// Si las preferencias no se pudieron leer, por qué. Se detecta igual, sin nombres ni carpetas a mano.
    pub preferencias: Option<ErrorNucleo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarpetaPerdida {
    pub raiz: PathBuf,
    pub nombre: Option<String>,
}

/// Qué pasó al añadir una carpeta a mano.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Alta {
    /// El usuario cerró el selector sin elegir.
    Cancelada,
    Anadida,
    /// Ya estaba, del registro o a mano, aunque se escribiera de otra forma.
    YaEstaba,
    /// No tiene `.build.info` en su raíz.
    NoEsInstalacion,
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

/// Instalaciones de este equipo, buscadas en todas las fuentes. Si las preferencias no se pudieron leer, se detecta
/// sin ellas y se dice por qué.
pub fn detectar_en_el_sistema(
    preferencias: Result<Preferencias, ErrorNucleo>,
) -> Result<Deteccion, ErrorNucleo> {
    detectar_con(&rutas_del_registro(), &rutas_del_agente(), preferencias)
}

fn detectar_con(
    registro: &[PathBuf],
    agente: &[PathBuf],
    preferencias: Result<Preferencias, ErrorNucleo>,
) -> Result<Deteccion, ErrorNucleo> {
    let (preferencias, error) = match preferencias {
        Ok(preferencias) => (preferencias, None),
        Err(error) => (Preferencias::default(), Some(error)),
    };
    let mut deteccion = detectar(registro, agente, &preferencias)?;
    deteccion.preferencias = error;
    Ok(deteccion)
}

/// Las raíces que la app encuentra sola (registro y agente de Battle.net). Al añadir una carpeta a mano, estas ya se
/// conocen.
pub fn rutas_automaticas() -> Vec<PathBuf> {
    let mut rutas = rutas_del_registro();
    rutas.extend(rutas_del_agente());
    rutas
}

/// Convierte las raíces del registro, del agente de Battle.net y las elegidas a mano en instalaciones, con los nombres
/// que les ha puesto el usuario. Una carpeta solo es una instalación si tiene `.build.info` [V]; las rutas repetidas
/// cuentan una vez y gana la primera fuente, en ese orden.
pub fn detectar(
    registro: &[PathBuf],
    agente: &[PathBuf],
    preferencias: &Preferencias,
) -> Result<Deteccion, ErrorNucleo> {
    let candidatas = registro
        .iter()
        .map(|raiz| (raiz, Fuente::Registro))
        .chain(agente.iter().map(|raiz| (raiz, Fuente::Battlenet)))
        .chain(
            preferencias
                .carpetas_manuales
                .iter()
                .map(|raiz| (raiz, Fuente::Manual)),
        );
    let mut vistas = HashSet::new();
    let mut deteccion = Deteccion {
        instalaciones: Vec::new(),
        no_encontradas: Vec::new(),
        preferencias: None,
    };
    for (raiz, fuente) in candidatas {
        if !vistas.insert(clave_de_ruta(raiz)) {
            continue;
        }
        let nombre = preferencias.nombre_instalacion(raiz);
        match (leer_instalacion(raiz, preferencias), fuente) {
            (Ok(Some(versiones)), _) => deteccion.instalaciones.push(Instalacion {
                raiz: raiz.clone(),
                fuente,
                nombre,
                versiones,
            }),
            // Una carpeta a mano que no se puede leer (unidad desconectada, permisos) no tumba la detección de las
            // demás: se lista como no encontrada.
            (Ok(None) | Err(_), Fuente::Manual) => {
                deteccion.no_encontradas.push(CarpetaPerdida {
                    raiz: raiz.clone(),
                    nombre,
                });
            }
            // El agente es una pista: una ruta suya que no es una instalación o no se puede leer se calla.
            (Ok(None), Fuente::Registro) | (Ok(None) | Err(_), Fuente::Battlenet) => {}
            (Err(error), Fuente::Registro) => return Err(error),
        }
    }
    Ok(deteccion)
}

/// Las versiones de la instalación de `raiz`, o `None` si no tiene `.build.info`.
fn leer_instalacion(
    raiz: &Path,
    preferencias: &Preferencias,
) -> Result<Option<Vec<VersionJuego>>, ErrorNucleo> {
    match leer_texto(&raiz.join(BUILD_INFO))? {
        Some(build_info) => leer_versiones(raiz, &build_info, preferencias).map(Some),
        None => Ok(None),
    }
}

/// Añade a las preferencias una carpeta elegida a mano si es una instalación y no se conocía ya (`conocidas`: las que
/// la app encuentra sola).
pub fn dar_de_alta(raiz: &Path, conocidas: &[PathBuf], preferencias: &mut Preferencias) -> Alta {
    let clave = clave_de_ruta(raiz);
    if conocidas.iter().any(|r| clave_de_ruta(r) == clave) || preferencias.es_manual(raiz) {
        return Alta::YaEstaba;
    }
    if !raiz.join(BUILD_INFO).is_file() {
        return Alta::NoEsInstalacion;
    }
    preferencias.anadir_carpeta(raiz.to_path_buf());
    Alta::Anadida
}

/// Las versiones son las subcarpetas con `.flavor.info` (`_retail_`, `_forever_`…).
fn leer_versiones(
    raiz: &Path,
    build_info: &str,
    preferencias: &Preferencias,
) -> Result<Vec<VersionJuego>, ErrorNucleo> {
    let mut versiones = Vec::new();
    for carpeta in subcarpetas(raiz)? {
        let Some(flavor_info) = leer_texto(&carpeta.ruta.join(FLAVOR_INFO))? else {
            continue;
        };
        let producto = valor_de(&flavor_info, COLUMNA_FLAVOR).unwrap_or_default();
        let mut version = leer_version(carpeta.nombre, &carpeta.ruta, producto, build_info)?;
        for cuenta in &mut version.cuentas.cuentas {
            cuenta.nombre = preferencias.nombre_cuenta(raiz, &version.carpeta, &cuenta.carpeta);
        }
        versiones.push(version);
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

/// `product.db` del agente de Battle.net, en `%ProgramData%` (spec 020). Sin la variable, la ruta de siempre.
fn rutas_del_agente() -> Vec<PathBuf> {
    let program_data = std::env::var_os("ProgramData")
        .map_or_else(|| PathBuf::from(r"C:\ProgramData"), PathBuf::from);
    agente_battlenet::raices(&program_data.join(r"Battle.net\Agent\product.db"))
}

/// Raíces que dice el registro: `Uninstall\World of Warcraft`, valor `InstallLocation` [V, en la vista de 32 bits
/// (`WOW6432Node`)]. Se mira también la de 64 bits; lo repetido lo junta `detectar`.
#[cfg(windows)]
pub fn rutas_del_registro() -> Vec<PathBuf> {
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
pub fn rutas_del_registro() -> Vec<PathBuf> {
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

    fn solo_registro(raiz: &Path) -> Vec<Instalacion> {
        detectar(&[raiz.to_path_buf()], &[], &Preferencias::default())
            .unwrap()
            .instalaciones
    }

    #[test]
    fn detecta_las_versiones_con_su_build_e_interfaz() {
        let tmp = tempfile::tempdir().unwrap();
        instalacion_de_prueba(tmp.path());

        let instalaciones = solo_registro(tmp.path());

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
        assert!(solo_registro(tmp.path()).is_empty());
    }

    #[test]
    fn la_misma_raiz_escrita_de_otra_forma_cuenta_una_vez() {
        let tmp = tempfile::tempdir().unwrap();
        instalacion_de_prueba(tmp.path());
        let otra_forma = PathBuf::from(format!(
            "{}\\",
            tmp.path().display().to_string().to_uppercase()
        ));

        let registro = [tmp.path().to_path_buf(), otra_forma];

        let deteccion = detectar(&registro, &[], &Preferencias::default()).unwrap();
        assert_eq!(deteccion.instalaciones.len(), 1);
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

        let instalacion = &solo_registro(tmp.path())[0];

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
            detectar(&[tmp.path().to_path_buf()], &[], &Preferencias::default()).unwrap(),
        )
        .unwrap();
        assert_eq!(json["noEncontradas"], serde_json::json!([]));
        assert_eq!(json["preferencias"], serde_json::Value::Null);
        let json = &json["instalaciones"];
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
        assert_eq!(json[0]["nombre"], serde_json::Value::Null);
        assert_eq!(
            json[0]["versiones"][0]["cuentas"]["cuentas"][0]["nombre"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn las_carpetas_a_mano_se_detectan_con_sus_nombres() {
        let tmp = tempfile::tempdir().unwrap();
        let registrada = tmp.path().join("registro");
        let manual = tmp.path().join("manual");
        for raiz in [&registrada, &manual] {
            create_dir_all(raiz).unwrap();
            instalacion_de_prueba(raiz);
        }
        let mut preferencias = Preferencias::default();
        preferencias.anadir_carpeta(manual.clone());
        preferencias.nombrar_instalacion(&manual, Some("Portátil".into()));
        preferencias.nombrar_cuenta(&manual, "_retail_", "1#1", Some("Alts".into()));

        let deteccion = detectar(&[registrada], &[], &preferencias).unwrap();

        let resumen: Vec<_> = deteccion
            .instalaciones
            .iter()
            .map(|i| (i.fuente, i.nombre.as_deref()))
            .collect();
        assert_eq!(
            resumen,
            [(Fuente::Registro, None), (Fuente::Manual, Some("Portátil"))]
        );
        let nombres: Vec<_> = deteccion.instalaciones[1]
            .versiones
            .iter()
            .map(|v| (v.carpeta.as_str(), v.cuentas.cuentas[0].nombre.as_deref()))
            .collect();
        assert_eq!(nombres, [("_forever_", None), ("_retail_", Some("Alts"))]);
        assert!(deteccion.no_encontradas.is_empty());
    }

    #[test]
    fn una_carpeta_a_mano_que_ya_no_esta_se_lista_como_no_encontrada() {
        let tmp = tempfile::tempdir().unwrap();
        let perdida = tmp.path().join("desaparecida");
        let mut preferencias = Preferencias::default();
        preferencias.anadir_carpeta(perdida.clone());
        preferencias.nombrar_instalacion(&perdida, Some("Casa".into()));
        // Del registro, una raíz sin `.build.info` no se lista: no es del usuario quitarla.
        let deteccion = detectar(&[tmp.path().join("vacia")], &[], &preferencias).unwrap();

        assert!(deteccion.instalaciones.is_empty());
        assert_eq!(
            deteccion.no_encontradas,
            [CarpetaPerdida {
                raiz: perdida,
                nombre: Some("Casa".into())
            }]
        );
    }

    #[test]
    fn la_misma_raiz_del_registro_y_a_mano_cuenta_una_vez_como_registro() {
        let tmp = tempfile::tempdir().unwrap();
        instalacion_de_prueba(tmp.path());
        let mut preferencias = Preferencias::default();
        preferencias.anadir_carpeta(tmp.path().to_path_buf());

        let deteccion = detectar(&[tmp.path().to_path_buf()], &[], &preferencias).unwrap();

        assert_eq!(deteccion.instalaciones.len(), 1);
        assert_eq!(deteccion.instalaciones[0].fuente, Fuente::Registro);
    }

    #[test]
    fn el_agente_es_la_segunda_fuente_y_se_junta_con_el_registro() {
        let tmp = tempfile::tempdir().unwrap();
        let comun = tmp.path().join("comun");
        let solo_agente = tmp.path().join("agente");
        for raiz in [&comun, &solo_agente] {
            create_dir_all(raiz).unwrap();
            instalacion_de_prueba(raiz);
        }
        // El agente escribe la ruta con `/`: tiene que casar con la del registro.
        let comun_con_barras = PathBuf::from(comun.display().to_string().replace('\\', "/"));
        let mut preferencias = Preferencias::default();
        preferencias.anadir_carpeta(solo_agente.clone());

        let deteccion = detectar(
            std::slice::from_ref(&comun),
            &[
                comun_con_barras,
                solo_agente.clone(),
                tmp.path().join("no-es-wow"),
            ],
            &preferencias,
        )
        .unwrap();

        let fuentes: Vec<_> = deteccion
            .instalaciones
            .iter()
            .map(|i| (i.raiz.clone(), i.fuente))
            .collect();
        assert_eq!(
            fuentes,
            [(comun, Fuente::Registro), (solo_agente, Fuente::Battlenet)]
        );
        assert!(deteccion.no_encontradas.is_empty());
    }

    #[test]
    fn preferencias_ilegibles_se_detecta_igual_y_se_dice() {
        let error = ErrorNucleo::PreferenciasIlegibles { ruta: "x".into() };
        let deteccion = detectar_con(&[], &[], Err(error.clone())).unwrap();
        assert_eq!(deteccion.preferencias, Some(error));
    }

    #[test]
    fn dar_de_alta_solo_acepta_instalaciones_nuevas() {
        let tmp = tempfile::tempdir().unwrap();
        let wow = tmp.path().join("wow");
        create_dir_all(&wow).unwrap();
        instalacion_de_prueba(&wow);
        let registrada = tmp.path().join("registrada");
        let mut preferencias = Preferencias::default();

        assert_eq!(
            dar_de_alta(tmp.path(), &[], &mut preferencias),
            Alta::NoEsInstalacion
        );
        assert_eq!(dar_de_alta(&wow, &[], &mut preferencias), Alta::Anadida);
        let otra_forma = PathBuf::from(format!("{}\\", wow.display().to_string().to_uppercase()));
        assert_eq!(
            dar_de_alta(&otra_forma, &[], &mut preferencias),
            Alta::YaEstaba
        );
        assert_eq!(
            dar_de_alta(
                &registrada,
                std::slice::from_ref(&registrada),
                &mut preferencias
            ),
            Alta::YaEstaba
        );
        assert_eq!(preferencias.carpetas_manuales, [wow]);
    }
}
