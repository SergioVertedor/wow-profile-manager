//! Piezas: qué fichero de `WTF` es cada pieza del formato y cómo se recorta. La comparten Exportación (de dónde se lee,
//! spec 023) e Importación (dónde se puede escribir, spec 024): una pieza solo va a **su** fichero.

use crate::paquetes::{Ambito, Categoria};
use serde::Serialize;
use std::collections::HashSet;

/// Los addons del propio cliente no están en `Interface/AddOns`, pero sí se pueden desactivar: siempre viajan.
pub const PREFIJO_BLIZZARD: &str = "blizzard_";
pub const ACTIVADO: &str = "enabled";
pub const DESACTIVADO: &str = "disabled";
const BOM: char = '\u{feff}';

/// Cómo se recorta una pieza antes de viajar, y cómo se fusiona al importar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filtro {
    Ninguno,
    /// `config-cache.wtf`: sin las claves de estado ni las de seguridad.
    Opciones,
    /// `AddOns.txt`: solo lo instalado y lo del cliente.
    Addons,
}

/// De dónde sale cada pieza. `origenes` en orden: vale el primero que exista.
#[derive(Debug)]
pub struct Fuente {
    pub id: &'static str,
    pub categoria: Categoria,
    pub origenes: &'static [Ambito],
    pub fichero: &'static str,
    pub destino: &'static str,
    pub filtro: Filtro,
}

const PERSONAJE: &[Ambito] = &[Ambito::Personaje];
const CUENTA: &[Ambito] = &[Ambito::Cuenta];

/// Las piezas de la entrega 1 de #56. Qué no viaja y por qué está en `docs/formato-wowprofile.md`.
pub const FUENTES: [Fuente; 10] = [
    fuente(
        "macros",
        Categoria::Macros,
        PERSONAJE,
        "macros-cache.txt",
        "personaje/macros-cache.txt",
    ),
    fuente(
        "macros-cuenta",
        Categoria::Macros,
        CUENTA,
        "macros-cache.txt",
        "cuenta/macros-cache.txt",
    ),
    // Un personaje sin teclas propias usa las de la cuenta [H]: viajan como teclas del personaje destino.
    fuente(
        "teclas",
        Categoria::Teclas,
        &[Ambito::Personaje, Ambito::Cuenta],
        "bindings-cache.wtf",
        "personaje/bindings-cache.wtf",
    ),
    fuente(
        "teclas-clic",
        Categoria::TeclasClic,
        PERSONAJE,
        "click-bindings-cache.txt",
        "personaje/click-bindings-cache.txt",
    ),
    filtrada(fuente(
        "opciones",
        Categoria::Opciones,
        PERSONAJE,
        "config-cache.wtf",
        "personaje/config-cache.wtf",
    )),
    filtrada(fuente(
        "opciones-cuenta",
        Categoria::Opciones,
        CUENTA,
        "config-cache.wtf",
        "cuenta/config-cache.wtf",
    )),
    fuente(
        "chat",
        Categoria::Chat,
        PERSONAJE,
        "chat-cache.txt",
        "personaje/chat-cache.txt",
    ),
    fuente(
        "ventanas",
        Categoria::Ventanas,
        PERSONAJE,
        "layout-local.txt",
        "personaje/layout-local.txt",
    ),
    fuente(
        "marcos-banda",
        Categoria::MarcosBanda,
        PERSONAJE,
        "CUFProfiles.txt",
        "personaje/CUFProfiles.txt",
    ),
    Fuente {
        filtro: Filtro::Addons,
        ..fuente(
            "addons-activos",
            Categoria::AddonsActivos,
            PERSONAJE,
            "AddOns.txt",
            "personaje/AddOns.txt",
        )
    },
];

const fn fuente(
    id: &'static str,
    categoria: Categoria,
    origenes: &'static [Ambito],
    fichero: &'static str,
    destino: &'static str,
) -> Fuente {
    Fuente {
        id,
        categoria,
        origenes,
        fichero,
        destino,
        filtro: Filtro::Ninguno,
    }
}

const fn filtrada(fuente: Fuente) -> Fuente {
    Fuente {
        filtro: Filtro::Opciones,
        ..fuente
    }
}

/// La fuente cuya categoría y `destino` son exactamente esos. Al importar, una entrada que no tiene ninguna no se
/// escribe: el paquete no puede mandar un fichero a un sitio que su categoría no explica (spec 024, decisión 7).
pub fn fuente_de_destino(categoria: Categoria, destino: &str) -> Option<&'static Fuente> {
    FUENTES
        .iter()
        .find(|f| f.categoria == categoria && f.destino == destino)
}

/// Por qué no viaja una línea. El orden es el de los grupos que se enseñan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Motivo {
    /// Quitaría al que importa un aviso de seguridad del juego.
    Seguridad,
    /// Estado de la partida, no configuración.
    Estado,
    /// Un addon que no está instalado.
    AddonNoInstalado,
}

/// Las claves que se quitan por un mismo motivo, en el orden del fichero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quitadas {
    pub motivo: Motivo,
    pub claves: Vec<String>,
}

/// Agrupa por motivo, en el orden de `Motivo` y sin grupos vacíos.
pub fn agrupar(quitadas: impl IntoIterator<Item = (Motivo, String)>) -> Vec<Quitadas> {
    let mut grupos: Vec<Quitadas> = Vec::new();
    for (motivo, clave) in quitadas {
        match grupos.iter_mut().find(|g| g.motivo == motivo) {
            Some(grupo) => grupo.claves.push(clave),
            None => grupos.push(Quitadas {
                motivo,
                claves: vec![clave],
            }),
        }
    }
    grupos.sort_by_key(|g| g.motivo);
    grupos
}

/// Todas las claves, en el orden de los grupos.
pub fn todas_las_claves(quitadas: &[Quitadas]) -> Vec<String> {
    quitadas.iter().flat_map(|g| g.claves.clone()).collect()
}

/// Claves de `config-cache.wtf` que no viajan (medidas el 2026-11-18). Los CVar no distinguen mayúsculas.
/// `AllowDangerousScripts` quitaría al que importa el aviso al ejecutar scripts; el resto es estado.
const CLAVE_SEGURIDAD: &str = "allowdangerousscripts";
const CLAVES_ESTADO: [&str; 7] = [
    "lastselectedclubid",
    "ejlootclass",
    "flaggedtutorials",
    "closedinfoframes",
    "closedinfoframesaccountwide",
    "numreputationheaders",
    "numcurrencycategories",
];
const CLAVES_PREFIJO: [&str; 5] = [
    "tracked",
    "hardtracked",
    "camerasaved",
    "perksactivities",
    "petjournal",
];
const CLAVE_CONTIENE: &str = "collapsed";

/// Quita las líneas enteras a las que `motivo` da un motivo. Las que quedan conservan sus bytes, salto de línea
/// incluido.
fn filtrar_lineas(
    bytes: &[u8],
    clave_de: impl Fn(&str) -> Option<String>,
    motivo: impl Fn(&str) -> Option<Motivo>,
) -> (Vec<u8>, Vec<Quitadas>) {
    let mut contenido = Vec::with_capacity(bytes.len());
    let mut quitadas = Vec::new();
    for linea in bytes.split_inclusive(|b| *b == b'\n') {
        let quitada = clave_de(&String::from_utf8_lossy(linea))
            .and_then(|clave| motivo(&clave).map(|m| (m, clave)));
        match quitada {
            Some(quitada) => quitadas.push(quitada),
            None => contenido.extend_from_slice(linea),
        }
    }
    (contenido, agrupar(quitadas))
}

/// `SET <clave> "<valor>"`: la clave. Las demás líneas no se tocan.
pub fn clave_de_opcion(linea: &str) -> Option<String> {
    let mut partes = sin_bom(linea).split_whitespace();
    let set = partes.next()?;
    set.eq_ignore_ascii_case("SET")
        .then(|| partes.next())
        .flatten()
        .map(str::to_owned)
}

/// Por qué no viaja una opción, o nada si viaja.
pub fn motivo_de_opcion(clave: &str) -> Option<Motivo> {
    let clave = clave.to_lowercase();
    if clave == CLAVE_SEGURIDAD {
        return Some(Motivo::Seguridad);
    }
    let estado = CLAVES_ESTADO.contains(&clave.as_str())
        || CLAVES_PREFIJO.iter().any(|p| clave.starts_with(p))
        || clave.contains(CLAVE_CONTIENE);
    estado.then_some(Motivo::Estado)
}

pub fn filtrar_opciones(bytes: &[u8]) -> (Vec<u8>, Vec<Quitadas>) {
    filtrar_lineas(bytes, clave_de_opcion, motivo_de_opcion)
}

pub fn filtrar_addons(bytes: &[u8], instalados: &HashSet<String>) -> (Vec<u8>, Vec<Quitadas>) {
    filtrar_lineas(
        bytes,
        |linea| addon_de_linea(linea).map(|(carpeta, _)| carpeta.to_owned()),
        |carpeta| {
            let carpeta = carpeta.to_lowercase();
            let instalado = carpeta.starts_with(PREFIJO_BLIZZARD) || instalados.contains(&carpeta);
            (!instalado).then_some(Motivo::AddonNoInstalado)
        },
    )
}

/// `<carpeta>: enabled|disabled` [V].
pub fn addon_de_linea(linea: &str) -> Option<(&str, &str)> {
    let (carpeta, valor) = sin_bom(linea).split_once(':')?;
    let carpeta = carpeta.trim();
    (!carpeta.is_empty()).then(|| (carpeta, valor.trim()))
}

pub fn lineas_addons(bytes: &[u8]) -> impl Iterator<Item = (String, String)> + '_ {
    bytes.split(|b| *b == b'\n').filter_map(|linea| {
        let texto = String::from_utf8_lossy(linea);
        addon_de_linea(&texto).map(|(c, v)| (c.to_owned(), v.to_owned()))
    })
}

/// Los ficheros medidos no traen BOM [V], pero si alguno lo trajera, la primera clave se tiene que reconocer igual.
pub fn sin_bom(linea: &str) -> &str {
    linea.trim_start_matches(BOM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opciones_sin_estado_ni_seguridad_y_con_los_bytes_del_resto() {
        let texto = b"SET nameplateMaxScale \"1\"\r\nSET hardTrackedQuests \"1 2\"\r\nSET AllowDangerousScripts \"1\"\r\n\
SET reputationsCollapsed \"x\"\r\nSET TrackedAchievements \"5\"\r\nSET cameraSavedPitch \"3\"\r\nSET showTimestamps \"%H\"";
        let (contenido, quitadas) = filtrar_opciones(texto);
        assert_eq!(
            contenido,
            b"SET nameplateMaxScale \"1\"\r\nSET showTimestamps \"%H\""
        );
        assert_eq!(
            quitadas,
            [
                grupo(Motivo::Seguridad, &["AllowDangerousScripts"]),
                grupo(
                    Motivo::Estado,
                    &[
                        "hardTrackedQuests",
                        "reputationsCollapsed",
                        "TrackedAchievements",
                        "cameraSavedPitch"
                    ]
                ),
            ]
        );
    }

    #[test]
    fn un_bom_al_principio_no_esconde_la_primera_clave() {
        let opciones = "\u{feff}SET AllowDangerousScripts \"1\"\r\nSET a \"1\"\r\n";
        let (contenido, quitadas) = filtrar_opciones(opciones.as_bytes());
        assert_eq!(
            quitadas,
            [grupo(Motivo::Seguridad, &["AllowDangerousScripts"])]
        );
        assert_eq!(contenido, b"SET a \"1\"\r\n");
        let addons = "\u{feff}Dominos: enabled\r\n";
        let instalados = HashSet::from(["dominos".to_owned()]);
        let (contenido, quitadas) = filtrar_addons(addons.as_bytes(), &instalados);
        assert!(quitadas.is_empty());
        assert_eq!(contenido, addons.as_bytes());
    }

    #[test]
    fn opciones_sin_nada_que_quitar_quedan_igual() {
        let texto = b"SET a \"1\"\nSET b \"2\"\n";
        assert_eq!(filtrar_opciones(texto), (texto.to_vec(), Vec::new()));
    }

    #[test]
    fn addons_solo_instalados_y_los_del_cliente() {
        let instalados = HashSet::from(["details".to_owned(), "dominos".to_owned()]);
        let texto = b"Dominos: enabled\r\nCarbonite: enabled\r\nBlizzard_CombatText: disabled\r\nDETAILS: disabled\r\nAtlasLoot: disabled\r\n";
        let (contenido, quitadas) = filtrar_addons(texto, &instalados);
        assert_eq!(
            contenido,
            b"Dominos: enabled\r\nBlizzard_CombatText: disabled\r\nDETAILS: disabled\r\n"
        );
        assert_eq!(
            quitadas,
            [grupo(Motivo::AddonNoInstalado, &["Carbonite", "AtlasLoot"])]
        );
    }

    #[test]
    fn los_grupos_van_en_el_orden_de_los_motivos_y_las_claves_en_el_del_fichero() {
        let grupos = agrupar([
            (Motivo::Estado, "b".to_owned()),
            (Motivo::Seguridad, "a".to_owned()),
            (Motivo::Estado, "c".to_owned()),
        ]);
        assert_eq!(
            grupos,
            [
                grupo(Motivo::Seguridad, &["a"]),
                grupo(Motivo::Estado, &["b", "c"])
            ]
        );
        assert_eq!(todas_las_claves(&grupos), ["a", "b", "c"]);
        assert!(agrupar([]).is_empty());
    }

    fn grupo(motivo: Motivo, claves: &[&str]) -> Quitadas {
        Quitadas {
            motivo,
            claves: claves.iter().map(|c| (*c).to_owned()).collect(),
        }
    }

    #[test]
    fn cada_categoria_solo_va_a_su_fichero() {
        assert_eq!(
            fuente_de_destino(Categoria::Macros, "cuenta/macros-cache.txt").map(|f| f.id),
            Some("macros-cuenta")
        );
        assert!(fuente_de_destino(Categoria::Macros, "personaje/config-cache.wtf").is_none());
        assert!(fuente_de_destino(Categoria::Chat, "cuenta/chat-cache.txt").is_none());
        let destinos: HashSet<_> = FUENTES.iter().map(|f| f.destino).collect();
        let ids: HashSet<_> = FUENTES.iter().map(|f| f.id).collect();
        assert_eq!(
            (destinos.len(), ids.len()),
            (FUENTES.len(), FUENTES.len()),
            "un destino y un id por pieza"
        );
    }
}
