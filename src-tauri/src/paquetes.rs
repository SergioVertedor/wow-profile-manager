//! Paquetes (`docs/arquitectura.md`): el `.wowprofile` según `docs/formato-wowprofile.md` (formato 1.0). Lo escribe
//! (spec 023) y lo lee con las comprobaciones del formato (`lectura`, spec 024). Solo escribe el `.wowprofile`.

mod lectura;

pub use lectura::{EntradaLeida, ManifiestoLeido, PaqueteLeido, leer_fichero};

use crate::error::ErrorNucleo;
use crate::escritura::escribir_atomico;
use crate::operaciones::sha256;
use crate::piezas::{Quitadas, todas_las_claves};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const NOMBRE_FORMATO: &str = "wowprofile";
const VERSION_FORMATO: &str = "1.0";
const MANIFIESTO: &str = "manifest.json";
const CARPETA_DATOS: &str = "datos/";
/// Límites del formato 1.0: piezas, bytes por pieza y bytes de todas juntas.
const MAX_PIEZAS: usize = 256;
pub const MAX_BYTES_PIEZA: usize = 32 * 1024 * 1024;
/// Texto que el formato reserva para sus marcadores: lo que ya lo trae no viaja.
pub const MARCADOR: &[u8] = b"@@WOWPROFILE:";
const MAX_BYTES_TOTAL: usize = 128 * 1024 * 1024;
/// El schema no admite más nombres en `claves_quitadas`.
const MAX_CLAVES_QUITADAS: usize = 256;

/// Lo que ve el usuario al elegir qué importa. Las que exporta la app; el formato tiene más (`modo-edicion`, #71).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Categoria {
    Macros,
    Teclas,
    TeclasClic,
    Opciones,
    Chat,
    Ventanas,
    MarcosBanda,
    AddonsActivos,
    ConfigAddon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tipo {
    Fichero,
    FicheroFiltrado,
    PerfilAcedb,
    Adaptador,
}

/// Dónde estaba la pieza en el origen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ambito {
    Personaje,
    Cuenta,
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

/// Un addon que el paquete menciona. El código del addon nunca viaja.
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

/// `$defs.idAddon` del schema: `^[A-Za-z0-9_!.\-]{1,64}$` y no solo puntos.
pub fn id_de_addon_valido(id: &str) -> bool {
    let caracteres = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '!' | '.' | '-'));
    caracteres && (1..=MAX_ID_ADDON).contains(&id.len()) && !id.chars().all(|c| c == '.')
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entrada {
    pub id: String,
    pub categoria: Categoria,
    pub tipo: Tipo,
    pub ambito: Ambito,
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

/// Un paquete listo para escribir: el manifiesto y el contenido de cada entrada, en el mismo orden.
#[derive(Debug)]
pub struct Paquete {
    pub manifiesto: Manifiesto,
    datos: Vec<Vec<u8>>,
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
        })
    }

    /// El `.wowprofile` en memoria: `manifest.json` y `datos/<id>`, nada más.
    pub fn a_zip(&self) -> Result<Vec<u8>, ErrorNucleo> {
        let fallo = |e: &dyn std::fmt::Display| ErrorNucleo::escritura(Path::new(MANIFIESTO), e);
        let manifiesto = serde_json::to_vec_pretty(&self.manifiesto).map_err(|e| fallo(&e))?;
        let opciones = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let ficheros = std::iter::once((MANIFIESTO.to_owned(), manifiesto.as_slice())).chain(
            self.manifiesto
                .entradas
                .iter()
                .zip(&self.datos)
                .map(|(e, d)| (format!("{CARPETA_DATOS}{}", e.id), d.as_slice())),
        );
        for (nombre, bytes) in ficheros {
            zip.start_file(nombre, opciones).map_err(|e| fallo(&e))?;
            zip.write_all(bytes).map_err(|e| fallo(&e))?;
        }
        Ok(zip.finish().map_err(|e| fallo(&e))?.into_inner())
    }

    /// Escribe el paquete en `ruta` a través de un temporal: un fallo no deja un `.wowprofile` a medias.
    pub fn guardar(&self, ruta: &Path) -> Result<(), ErrorNucleo> {
        escribir_atomico(ruta, &self.a_zip()?)
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
        ambito: pieza.ambito,
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
    use std::io::Read;

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
        assert_eq!(nombres, ["manifest.json", "datos/macros", "datos/chat"]);
        assert_eq!(entradas[1].1, b"VER 3\r\n");
        let manifiesto: serde_json::Value = serde_json::from_slice(&entradas[0].1).unwrap();
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
        assert_ne!(entradas[0].1.first(), Some(&0xEF), "sin BOM");
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
