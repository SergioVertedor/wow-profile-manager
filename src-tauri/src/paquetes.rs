//! Paquetes (`docs/arquitectura.md`): el `.wowprofile` según `docs/formato-wowprofile.md` (formato 1.0, y 1.1 si
//! lleva addons, spec 036). Lo escribe (spec 023) y lo lee con las comprobaciones del formato (`lectura`, spec 024).
//! Solo escribe el `.wowprofile`.

mod lectura;

use lectura::id_de_entrada_valido;
pub use lectura::{EntradaLeida, ManifiestoLeido, PaqueteLeido, leer_carpeta, leer_fichero};

use crate::addon_zip;
use crate::error::ErrorNucleo;
use crate::escritura::escribir_atomico_con;
use crate::operaciones::{segmento_literal, sha256};
use crate::piezas::{Quitadas, todas_las_claves};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{BufWriter, Seek, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const NOMBRE_FORMATO: &str = "wowprofile";
const VERSION_FORMATO: &str = "1.0";
/// La de los paquetes que llevan addons. Uno sin addons sigue siendo 1.0.
const VERSION_FORMATO_ADDONS: &str = "1.1";
const MANIFIESTO: &str = "manifest.json";
const CARPETA_DATOS: &str = "datos/";
/// Límites del formato 1.0: piezas, bytes por pieza y bytes de todas juntas.
const MAX_PIEZAS: usize = 256;
pub const MAX_BYTES_PIEZA: usize = 32 * 1024 * 1024;
/// Texto que el formato reserva para sus marcadores: lo que ya lo trae no viaja.
pub const MARCADOR: &[u8] = b"@@WOWPROFILE:";
const MAX_BYTES_TOTAL: usize = 128 * 1024 * 1024;
/// Las piezas de addon juntas (formato 1.1). Van aparte de las 128 MiB: no se cargan todas en memoria.
pub const MAX_BYTES_ADDONS: u64 = 512 * 1024 * 1024;
/// El `destino` de una pieza de addon es `addons/<carpeta>`.
pub const PREFIJO_DESTINO_ADDONS: &str = "addons/";
/// El schema no admite más nombres en `claves_quitadas`.
const MAX_CLAVES_QUITADAS: usize = 256;

/// Lo que ve el usuario al elegir qué importa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Categoria {
    Macros,
    Teclas,
    TeclasClic,
    Opciones,
    Chat,
    Ventanas,
    /// El diseño del Modo Edición que usa el personaje (spec 034).
    ModoEdicion,
    MarcosBanda,
    AddonsActivos,
    ConfigAddon,
    /// La carpeta de un addon, con su código (formato 1.1, spec 036).
    Addon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tipo {
    Fichero,
    FicheroFiltrado,
    PerfilAcedb,
    Adaptador,
    /// Un ZIP con los ficheros de la carpeta de un addon.
    Carpeta,
}

/// Dónde estaba una pieza de configuración en el origen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ambito {
    Personaje,
    Cuenta,
}

/// El `ambito` del manifiesto: el de la configuración o, en una pieza de addon, la instalación (`Interface/AddOns` es
/// de todas las cuentas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AmbitoEntrada {
    Personaje,
    Cuenta,
    Instalacion,
}

impl From<Ambito> for AmbitoEntrada {
    fn from(ambito: Ambito) -> Self {
        match ambito {
            Ambito::Personaje => Self::Personaje,
            Ambito::Cuenta => Self::Cuenta,
        }
    }
}

/// La carpeta de un addon que viaja en el paquete. Se empaqueta al guardar, leyendo del disco en ese momento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarpetaAddon {
    pub id: String,
    /// Nombre de la carpeta en `Interface/AddOns`, ya validado como `id` de addon.
    pub carpeta: String,
    pub origen: PathBuf,
}

/// Una pieza con su contenido, tal como sale de Exportación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pieza {
    pub id: String,
    pub categoria: Categoria,
    pub tipo: Tipo,
    pub ambito: Ambito,
    /// Ruta lógica (`personaje/<fichero>` o `cuenta/[SavedVariables/]<fichero>`), nunca una ruta de disco.
    pub destino: String,
    pub contenido: Vec<u8>,
    /// Solo en `fichero-filtrado`: lo que se quitó (claves, carpetas de addons o variables), por motivo.
    pub quitadas: Vec<Quitadas>,
    /// Solo en `config-addon`: de qué addon, variable, perfil y adaptador es.
    pub addon: Option<DeAddon>,
}

/// Lo que una pieza `config-addon` dice de su addon en el manifiesto.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeAddon {
    pub addon: String,
    pub variable: Option<String>,
    pub perfil: Option<String>,
    pub adaptador: Option<RefAdaptador>,
    pub marcadores: Vec<Marcador>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefAdaptador {
    pub id: String,
    pub version: u32,
}

/// Dónde lleva la pieza un marcador del formato.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Marcador {
    Personaje,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Formato {
    pub nombre: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Perfil {
    pub nombre: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descripcion: Option<String>,
    /// RFC 3339 en UTC.
    pub creado: String,
}

/// Sin `clase`: no se puede leer de forma fiable de los ficheros (spec 023, decisión 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Origen {
    pub producto: &'static str,
    pub build: String,
    pub interfaz: u32,
}

/// Un addon que el paquete menciona. Su código solo viaja si además hay una pieza `addon` suya.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddonPaquete {
    pub id: String,
    pub titulo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enlace: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependencias: Vec<String>,
}

/// Límites de `addons[]` en el schema 1.0.
const MAX_TITULO: usize = 120;
const MAX_VERSION: usize = 64;
const MAX_ENLACE: usize = 300;
const MAX_DEPENDENCIAS: usize = 64;
const MAX_ID_ADDON: usize = 64;

impl AddonPaquete {
    /// Lo que dice el `.toc`, dentro de lo que admite el schema. Un addon cuya carpeta no es un `id` válido no se
    /// describe (`None`); lo demás que no cabe se recorta o se deja fuera, nunca se inventa.
    pub fn nuevo(
        carpeta: &str,
        titulo: Option<&str>,
        version: Option<&str>,
        enlace: Option<&str>,
        dependencias: &[String],
    ) -> Option<Self> {
        if !id_de_addon_valido(carpeta) {
            return None;
        }
        let titulo = titulo
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .unwrap_or(carpeta);
        Some(Self {
            id: carpeta.to_owned(),
            titulo: titulo.chars().take(MAX_TITULO).collect(),
            version: version
                .filter(|v| v.chars().count() <= MAX_VERSION)
                .map(str::to_owned),
            enlace: enlace
                .filter(|e| e.starts_with("https://") && e.chars().count() <= MAX_ENLACE)
                .map(str::to_owned),
            dependencias: dependencias
                .iter()
                .filter(|d| id_de_addon_valido(d))
                .take(MAX_DEPENDENCIAS)
                .cloned()
                .collect(),
        })
    }
}

/// `$defs.idAddon` del schema: `^[A-Za-z0-9_!.\-]{1,64}$`, no solo puntos y un nombre de carpeta que Windows escribe
/// tal cual (sin `CON`, `NUL`…, ni punto final): con la 1.1 es la carpeta que se crea en `Interface/AddOns`.
pub fn id_de_addon_valido(id: &str) -> bool {
    let caracteres = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '!' | '.' | '-'));
    caracteres
        && (1..=MAX_ID_ADDON).contains(&id.len())
        && !id.chars().all(|c| c == '.')
        && segmento_literal(id)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entrada {
    pub id: String,
    pub categoria: Categoria,
    pub tipo: Tipo,
    pub ambito: AmbitoEntrada,
    pub destino: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addon: Option<String>,
    pub sha256: String,
    pub bytes: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub marcadores: Vec<Marcador>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claves_quitadas: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perfil: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptador: Option<RefAdaptador>,
}

/// `manifest.json`. Los tipos no tienen campos de cuenta, reino, personaje ni rutas: no se pueden colar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Manifiesto {
    pub formato: Formato,
    pub perfil: Perfil,
    pub origen: Origen,
    pub addons: Vec<AddonPaquete>,
    pub entradas: Vec<Entrada>,
}

/// Un paquete listo para escribir: el manifiesto y el contenido de cada entrada, en el mismo orden, y las carpetas de
/// addons, que se empaquetan al escribir (sus entradas se añaden entonces al manifiesto que se escribe).
#[derive(Debug)]
pub struct Paquete {
    pub manifiesto: Manifiesto,
    datos: Vec<Vec<u8>>,
    carpetas: Vec<CarpetaAddon>,
}

impl Paquete {
    /// Calcula hash y tamaño de cada pieza y comprueba los límites del formato.
    pub fn nuevo(
        perfil: Perfil,
        origen: Origen,
        addons: Vec<AddonPaquete>,
        piezas: Vec<Pieza>,
    ) -> Result<Self, ErrorNucleo> {
        let total: usize = piezas.iter().map(|p| p.contenido.len()).sum();
        let grande = piezas.iter().any(|p| p.contenido.len() > MAX_BYTES_PIEZA);
        if piezas.len() > MAX_PIEZAS || grande || total > MAX_BYTES_TOTAL {
            return Err(ErrorNucleo::PaqueteDemasiadoGrande {});
        }
        let (entradas, datos) = piezas.into_iter().map(entrada_de).unzip();
        Ok(Self {
            manifiesto: Manifiesto {
                formato: Formato {
                    nombre: NOMBRE_FORMATO,
                    version: VERSION_FORMATO,
                },
                perfil,
                origen,
                addons,
                entradas,
            },
            datos,
            carpetas: Vec::new(),
        })
    }

    /// Añade carpetas de addons: el paquete pasa a ser 1.1. Su tamaño se comprueba al empaquetarlas.
    pub fn con_carpetas(mut self, carpetas: Vec<CarpetaAddon>) -> Result<Self, ErrorNucleo> {
        if self.manifiesto.entradas.len() + carpetas.len() > MAX_PIEZAS {
            return Err(ErrorNucleo::PaqueteDemasiadoGrande {});
        }
        let mut ids: HashSet<&str> = self
            .manifiesto
            .entradas
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        if let Some(mala) = carpetas.iter().find(|c| {
            !id_de_addon_valido(&c.carpeta) || !id_de_entrada_valido(&c.id) || !ids.insert(&c.id)
        }) {
            return Err(ErrorNucleo::AddonNoEmpaquetado {
                addon: mala.carpeta.clone(),
                detalle: format!("id no válido o repetido: {}", mala.id),
            });
        }
        if !carpetas.is_empty() {
            self.manifiesto.formato.version = VERSION_FORMATO_ADDONS;
        }
        self.carpetas = carpetas;
        Ok(self)
    }

    /// El `.wowprofile` en memoria, para los tests.
    #[cfg(test)]
    pub fn a_zip(&self) -> Result<Vec<u8>, ErrorNucleo> {
        self.escribir(std::io::Cursor::new(Vec::new()), Path::new(MANIFIESTO))
            .map(std::io::Cursor::into_inner)
    }

    /// Escribe el paquete en `ruta` a través de un temporal: un fallo no deja un `.wowprofile` a medias. Las carpetas
    /// de addons se empaquetan de una en una, sin tener el paquete entero en memoria.
    pub fn guardar(&self, ruta: &Path) -> Result<(), ErrorNucleo> {
        escribir_atomico_con(ruta, |fichero| {
            self.escribir(BufWriter::new(fichero), ruta)?
                .into_inner()
                .map(|_| ())
                .map_err(|e| ErrorNucleo::escritura(ruta, e.error()))
        })
    }

    /// `datos/<id>` de cada entrada, las carpetas de addons y, al final, `manifest.json`: ya con las entradas de las
    /// carpetas, cuyo hash solo se sabe al empaquetarlas. Nada más.
    fn escribir<W: Write + Seek>(&self, salida: W, ruta: &Path) -> Result<W, ErrorNucleo> {
        let fallo = |e: &dyn std::fmt::Display| ErrorNucleo::escritura(ruta, e);
        let comprimido =
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        // Un ZIP de addon ya va comprimido: comprimirlo otra vez solo gasta tiempo.
        let tal_cual = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let mut zip = ZipWriter::new(salida);
        let mut anadir = |nombre: String, bytes: &[u8], opciones: SimpleFileOptions| {
            zip.start_file(nombre, opciones)
                .and_then(|()| zip.write_all(bytes).map_err(Into::into))
                .map_err(|e| fallo(&e))
        };
        for (entrada, bytes) in self.manifiesto.entradas.iter().zip(&self.datos) {
            anadir(format!("{CARPETA_DATOS}{}", entrada.id), bytes, comprimido)?;
        }
        let mut manifiesto = self.manifiesto.clone();
        let mut total: u64 = 0;
        for carpeta in &self.carpetas {
            let bytes = addon_zip::empaquetar(&carpeta.origen).map_err(|e| {
                ErrorNucleo::AddonNoEmpaquetado {
                    addon: carpeta.carpeta.clone(),
                    detalle: e.to_string(),
                }
            })?;
            total += bytes.len() as u64;
            if total > MAX_BYTES_ADDONS {
                return Err(ErrorNucleo::PaqueteDemasiadoGrande {});
            }
            anadir(format!("{CARPETA_DATOS}{}", carpeta.id), &bytes, tal_cual)?;
            manifiesto
                .entradas
                .push(entrada_de_carpeta(carpeta, &bytes));
        }
        let json = serde_json::to_vec_pretty(&manifiesto).map_err(|e| fallo(&e))?;
        anadir(MANIFIESTO.to_owned(), &json, comprimido)?;
        zip.finish().map_err(|e| fallo(&e))
    }
}

fn entrada_de_carpeta(carpeta: &CarpetaAddon, bytes: &[u8]) -> Entrada {
    Entrada {
        id: carpeta.id.clone(),
        categoria: Categoria::Addon,
        tipo: Tipo::Carpeta,
        ambito: AmbitoEntrada::Instalacion,
        destino: format!("{PREFIJO_DESTINO_ADDONS}{}", carpeta.carpeta),
        addon: Some(carpeta.carpeta.clone()),
        sha256: sha256(bytes),
        bytes: bytes.len() as u64,
        marcadores: Vec::new(),
        claves_quitadas: None,
        variable: None,
        perfil: None,
        adaptador: None,
    }
}

fn entrada_de(pieza: Pieza) -> (Entrada, Vec<u8>) {
    let claves_quitadas = (pieza.tipo == Tipo::FicheroFiltrado).then(|| {
        let mut claves = todas_las_claves(&pieza.quitadas);
        claves.truncate(MAX_CLAVES_QUITADAS);
        claves
    });
    let de_addon = pieza.addon.unwrap_or_default();
    let entrada = Entrada {
        id: pieza.id,
        categoria: pieza.categoria,
        tipo: pieza.tipo,
        ambito: pieza.ambito.into(),
        destino: pieza.destino,
        addon: (!de_addon.addon.is_empty()).then_some(de_addon.addon),
        sha256: sha256(&pieza.contenido),
        bytes: pieza.contenido.len() as u64,
        marcadores: de_addon.marcadores,
        claves_quitadas,
        variable: de_addon.variable,
        perfil: de_addon.perfil,
        adaptador: de_addon.adaptador,
    };
    (entrada, pieza.contenido)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{Cursor, Read};

    pub fn pieza(id: &'static str, contenido: &[u8]) -> Pieza {
        Pieza {
            id: id.to_owned(),
            categoria: Categoria::Macros,
            tipo: Tipo::Fichero,
            ambito: Ambito::Personaje,
            destino: "personaje/macros-cache.txt".to_owned(),
            contenido: contenido.to_vec(),
            quitadas: Vec::new(),
            addon: None,
        }
    }

    fn perfil() -> Perfil {
        Perfil {
            nombre: "Druida".into(),
            descripcion: None,
            creado: "2026-11-18T10:00:00Z".into(),
        }
    }

    fn origen() -> Origen {
        Origen {
            producto: "wowf",
            build: "1.60.1.70938".into(),
            interfaz: 16001,
        }
    }

    /// Lee un ZIP entero: nombre y contenido de cada entrada, en orden.
    pub fn entradas_zip(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        (0..zip.len())
            .map(|i| {
                let mut f = zip.by_index(i).unwrap();
                let mut contenido = Vec::new();
                f.read_to_end(&mut contenido).unwrap();
                (f.name().unwrap().into_owned(), contenido)
            })
            .collect()
    }

    #[test]
    fn el_zip_lleva_el_manifiesto_y_una_pieza_por_entrada_y_nada_mas() {
        let paquete = Paquete::nuevo(
            perfil(),
            origen(),
            Vec::new(),
            vec![pieza("macros", b"VER 3\r\n"), pieza("chat", b"")],
        )
        .unwrap();

        let entradas = entradas_zip(&paquete.a_zip().unwrap());

        let nombres: Vec<_> = entradas.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(nombres, ["datos/macros", "datos/chat", "manifest.json"]);
        assert_eq!(entradas[0].1, b"VER 3\r\n");
        let manifiesto: serde_json::Value = serde_json::from_slice(&entradas[2].1).unwrap();
        assert_eq!(
            manifiesto["formato"],
            serde_json::json!({ "nombre": "wowprofile", "version": "1.0" })
        );
        assert_eq!(manifiesto["origen"]["interfaz"], 16001);
        assert_eq!(manifiesto["entradas"][0]["bytes"], 7);
        assert_eq!(
            manifiesto["entradas"][0]["sha256"],
            sha256(b"VER 3\r\n").as_str()
        );
        assert!(manifiesto["perfil"].get("descripcion").is_none());
        assert!(manifiesto["entradas"][0].get("claves_quitadas").is_none());
        assert_ne!(entradas[2].1.first(), Some(&0xEF), "sin BOM");
    }

    #[test]
    fn una_pieza_filtrada_lleva_sus_claves_quitadas_aunque_no_haya_ninguna() {
        let mut filtrada = pieza("opciones", b"");
        filtrada.tipo = Tipo::FicheroFiltrado;
        let paquete = Paquete::nuevo(perfil(), origen(), Vec::new(), vec![filtrada]).unwrap();
        let json = serde_json::to_value(&paquete.manifiesto).unwrap();
        assert_eq!(
            json["entradas"][0]["claves_quitadas"],
            serde_json::json!([])
        );
    }

    #[test]
    fn pasarse_de_los_limites_no_arma_el_paquete() {
        let muchas = (0..=MAX_PIEZAS).map(|_| pieza("macros", b"")).collect();
        let grande = vec![pieza("macros", &vec![0; MAX_BYTES_PIEZA + 1])];
        let justa = vec![pieza("macros", &vec![0; MAX_BYTES_PIEZA])];
        for piezas in [muchas, grande] {
            assert_eq!(
                Paquete::nuevo(perfil(), origen(), Vec::new(), piezas).unwrap_err(),
                ErrorNucleo::PaqueteDemasiadoGrande {}
            );
        }
        assert!(Paquete::nuevo(perfil(), origen(), Vec::new(), justa).is_ok());
    }

    #[test]
    fn un_addon_se_describe_dentro_de_lo_que_admite_el_schema() {
        let dependencias = ["Ace3".to_owned(), "Con espacio".to_owned()];
        let largo = "t".repeat(MAX_TITULO + 5);
        let addon = AddonPaquete::nuevo(
            "Prat-3.0",
            Some(&largo),
            Some(&"v".repeat(MAX_VERSION + 1)),
            Some("https://ejemplo.org"),
            &dependencias,
        )
        .unwrap();
        assert_eq!(addon.titulo.chars().count(), MAX_TITULO);
        assert_eq!(addon.version, None);
        assert_eq!(addon.enlace.as_deref(), Some("https://ejemplo.org"));
        assert_eq!(addon.dependencias, ["Ace3"]);

        let sin_titulo =
            AddonPaquete::nuevo("Dominos", Some("  "), None, Some("http://x"), &[]).unwrap();
        assert_eq!(
            (sin_titulo.titulo.as_str(), sin_titulo.enlace),
            ("Dominos", None)
        );
        for no_valido in ["Mi Addon", "..", "", &"a".repeat(MAX_ID_ADDON + 1)] {
            assert_eq!(
                AddonPaquete::nuevo(no_valido, None, None, None, &[]),
                None,
                "{no_valido}"
            );
        }
    }

    #[test]
    fn guardar_escribe_el_zip_en_la_ruta() {
        let tmp = tempfile::tempdir().unwrap();
        let ruta = tmp.path().join("perfil.wowprofile");
        let paquete =
            Paquete::nuevo(perfil(), origen(), Vec::new(), vec![pieza("macros", b"x")]).unwrap();
        paquete.guardar(&ruta).unwrap();
        assert_eq!(entradas_zip(&std::fs::read(&ruta).unwrap()).len(), 2);
    }
}

#[cfg(test)]
mod tests_carpetas;

#[cfg(test)]
mod config_addon {
    use super::tests::pieza;
    use super::*;

    #[test]
    fn las_piezas_de_addon_se_vuelven_a_leer_con_sus_campos() {
        let perfil_acedb = Pieza {
            id: "addon-dominos-perfil-dominosdb".into(),
            categoria: Categoria::ConfigAddon,
            tipo: Tipo::PerfilAcedb,
            ambito: Ambito::Cuenta,
            destino: "cuenta/SavedVariables/Dominos.lua".into(),
            contenido: b"{\n[\"profile\"] = {\n},\n}\n".to_vec(),
            quitadas: Vec::new(),
            addon: Some(DeAddon {
                addon: "Dominos".into(),
                variable: Some("DominosDB".into()),
                perfil: Some("@@WOWPROFILE:PERSONAJE@@".into()),
                adaptador: None,
                marcadores: vec![Marcador::Personaje],
            }),
        };
        let adaptador = Pieza {
            id: "addon-wim-wim3-data".into(),
            tipo: Tipo::Adaptador,
            destino: "cuenta/SavedVariables/WIM.lua".into(),
            contenido: b"WIM3_Data = {\n}\n".to_vec(),
            addon: Some(DeAddon {
                addon: "WIM".into(),
                variable: Some("WIM3_Data".into()),
                adaptador: Some(RefAdaptador {
                    id: "wim".into(),
                    version: 2,
                }),
                ..DeAddon::default()
            }),
            ..perfil_acedb.clone()
        };
        let perfil = Perfil {
            nombre: "Druida".into(),
            descripcion: None,
            creado: "2026-11-18T10:00:00Z".into(),
        };
        let origen = Origen {
            producto: "wowf",
            build: "1.60.1.70938".into(),
            interfaz: 16001,
        };
        let piezas = vec![pieza("macros", b"VER 3\r\n"), perfil_acedb, adaptador];
        let tmp = tempfile::tempdir().unwrap();
        let ruta = tmp.path().join("p.wowprofile");
        Paquete::nuevo(perfil, origen, Vec::new(), piezas)
            .unwrap()
            .guardar(&ruta)
            .unwrap();

        let leido = leer_fichero(&ruta).unwrap();

        let entradas = &leido.manifiesto.entradas;
        assert_eq!(entradas[1].categoria, "config-addon");
        assert_eq!(entradas[1].tipo, "perfil-acedb");
        assert_eq!(entradas[1].addon.as_deref(), Some("Dominos"));
        assert_eq!(entradas[1].variable.as_deref(), Some("DominosDB"));
        assert_eq!(
            entradas[1].perfil.as_deref(),
            Some("@@WOWPROFILE:PERSONAJE@@")
        );
        assert_eq!(entradas[1].marcadores, Some(vec!["PERSONAJE".to_owned()]));
        assert_eq!(entradas[2].tipo, "adaptador");
        assert!(entradas[2].adaptador.is_some());
        assert_eq!(entradas[2].perfil, None);
    }
}
