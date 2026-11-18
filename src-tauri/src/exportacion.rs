//! Exportación (`docs/arquitectura.md`): arma las piezas de un personaje para un `.wowprofile` (spec 023, entrega 1 de
//! #56: los ficheros que se copian sin leer Lua). Solo lee; el paquete lo escribe Paquetes.

use crate::addons::Addon;
use crate::deteccion::{self, CARPETA_ACCOUNT, CARPETA_WTF, PersonajeUbicado};
use crate::error::ErrorNucleo;
use crate::operaciones::{ahora_rfc3339, segmento_literal};
use crate::paquetes::{AddonPaquete, Ambito, Categoria, Origen, Paquete, Perfil, Pieza, Tipo};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Texto que el formato reserva para sus marcadores: una pieza que ya lo trae no viaja.
const MARCADOR: &[u8] = b"@@WOWPROFILE:";
/// Los addons del propio cliente no están en `Interface/AddOns`, pero sí se pueden desactivar: siempre viajan.
const PREFIJO_BLIZZARD: &str = "blizzard_";
const ACTIVADO: &str = "enabled";
const BOM: char = '\u{feff}';
const MAX_NOMBRE: usize = 80;
const MAX_DESCRIPCION: usize = 500;
pub const EXTENSION: &str = "wowprofile";
/// Caracteres que Windows no admite en un nombre de fichero.
const NO_ADMITIDOS_EN_FICHERO: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Cómo se recorta una pieza antes de viajar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filtro {
    Ninguno,
    /// `config-cache.wtf`: sin las claves de estado ni las de seguridad.
    Opciones,
    /// `AddOns.txt`: solo lo instalado y lo del cliente.
    Addons,
}

/// De dónde sale cada pieza. `origenes` en orden: vale el primero que exista.
struct Fuente {
    id: &'static str,
    categoria: Categoria,
    origenes: &'static [Ambito],
    fichero: &'static str,
    destino: &'static str,
    filtro: Filtro,
}

const PERSONAJE: &[Ambito] = &[Ambito::Personaje];
const CUENTA: &[Ambito] = &[Ambito::Cuenta];

/// Las piezas de la entrega 1. Qué no viaja y por qué está en `docs/formato-wowprofile.md`.
const FUENTES: [Fuente; 10] = [
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

/// Claves de `config-cache.wtf` que no viajan (medidas el 2026-11-18). Los CVar no distinguen mayúsculas.
/// `AllowDangerousScripts` quitaría al que importa el aviso al ejecutar scripts; el resto es estado.
const CLAVES_EXACTAS: [&str; 8] = [
    "allowdangerousscripts",
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

/// Lo que ve el diálogo de exportar: sin el contenido.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Propuesta {
    pub piezas: Vec<PiezaPropuesta>,
    /// Ids de las piezas que ya traen `@@WOWPROFILE:` y no viajan.
    pub apartadas: Vec<String>,
    pub juego_abierto: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiezaPropuesta {
    pub id: &'static str,
    pub categoria: Categoria,
    pub ambito: Ambito,
    pub bytes: u64,
    /// Líneas que se quitan al filtrar.
    pub quitadas: usize,
}

/// Lo que el jugador pide exportar.
pub struct Pedido {
    pub seleccion: Vec<String>,
    pub nombre: String,
    pub descripcion: String,
}

/// Qué se puede exportar del personaje ahora mismo, para el diálogo.
pub fn propuesta_de(
    personaje: &PersonajeUbicado,
    juego_abierto: bool,
) -> Result<Propuesta, ErrorNucleo> {
    let addons = deteccion::addons_de(&personaje.raiz_version, personaje.producto)?;
    let (piezas, apartadas) = piezas(personaje, &addons.addons)?;
    Ok(propuesta(&piezas, apartadas, juego_abierto))
}

/// El paquete con lo pedido, leído del disco en este momento (lo que se guarda es lo que hay al guardar).
pub fn paquete(personaje: &PersonajeUbicado, pedido: &Pedido) -> Result<Paquete, ErrorNucleo> {
    let (Some(build), Some(interfaz)) =
        deteccion::origen_de(&personaje.raiz_version, personaje.producto)?
    else {
        return Err(ErrorNucleo::ExportacionSinVersion {});
    };
    let addons = deteccion::addons_de(&personaje.raiz_version, personaje.producto)?;
    let (todas, _) = piezas(personaje, &addons.addons)?;
    let (elegidas, perfil) = elegir(todas, pedido, ahora_rfc3339())?;
    let origen = Origen {
        producto: personaje.producto.codigo(),
        build,
        interfaz,
    };
    let addons = addons_del_paquete(&elegidas, &addons.addons);
    Paquete::nuevo(perfil, origen, addons, elegidas)
}

/// `<nombre>.wowprofile` sin los caracteres que Windows no admite. El nombre ya viene validado (no vacío).
pub fn nombre_de_fichero(nombre: &str) -> String {
    let limpio: String = nombre
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || NO_ADMITIDOS_EN_FICHERO.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let limpio = limpio.trim_end_matches(['.', ' ']);
    let limpio = if limpio.is_empty() { "perfil" } else { limpio };
    // `CON.wowprofile` también es el dispositivo `CON` para Windows.
    let sufijo = if segmento_literal(limpio) { "" } else { "_" };
    format!("{limpio}{sufijo}.{EXTENSION}")
}

/// La ruta elegida, con la extensión del formato si el usuario la quitó.
pub fn con_extension(ruta: PathBuf) -> PathBuf {
    let tiene = ruta
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION));
    if tiene {
        return ruta;
    }
    let mut texto = ruta.into_os_string();
    texto.push(format!(".{EXTENSION}"));
    PathBuf::from(texto)
}

/// Las piezas que tiene ahora en disco el personaje, sin las apartadas por el marcador (que van aparte).
fn piezas(
    personaje: &PersonajeUbicado,
    addons: &[Addon],
) -> Result<(Vec<Pieza>, Vec<String>), ErrorNucleo> {
    let instalados: HashSet<String> = addons.iter().map(|a| a.carpeta.to_lowercase()).collect();
    let mut piezas = Vec::new();
    let mut apartadas = Vec::new();
    for fuente in &FUENTES {
        let Some(pieza) = leer_pieza(personaje, fuente, &instalados)? else {
            continue;
        };
        if contiene(&pieza.contenido, MARCADOR) {
            apartadas.push(pieza.id.to_owned());
        } else {
            piezas.push(pieza);
        }
    }
    Ok((piezas, apartadas))
}

fn propuesta(piezas: &[Pieza], apartadas: Vec<String>, juego_abierto: bool) -> Propuesta {
    Propuesta {
        piezas: piezas
            .iter()
            .map(|p| PiezaPropuesta {
                id: p.id,
                categoria: p.categoria,
                ambito: p.ambito,
                bytes: p.contenido.len() as u64,
                quitadas: p.claves_quitadas.len(),
            })
            .collect(),
        apartadas,
        juego_abierto,
    }
}

/// Se queda con las piezas pedidas y valida nombre y descripción.
fn elegir(
    piezas: Vec<Pieza>,
    pedido: &Pedido,
    creado: String,
) -> Result<(Vec<Pieza>, Perfil), ErrorNucleo> {
    let nombre = pedido.nombre.trim();
    let descripcion = pedido.descripcion.trim();
    let largo = |t: &str| t.chars().count();
    if nombre.is_empty() || largo(nombre) > MAX_NOMBRE || largo(descripcion) > MAX_DESCRIPCION {
        return Err(ErrorNucleo::PerfilNoValido {});
    }
    let elegidas: Vec<_> = piezas
        .into_iter()
        .filter(|p| pedido.seleccion.iter().any(|s| s == p.id))
        .collect();
    if elegidas.is_empty() {
        return Err(ErrorNucleo::SeleccionVacia {});
    }
    let perfil = Perfil {
        nombre: nombre.to_owned(),
        descripcion: (!descripcion.is_empty()).then(|| descripcion.to_owned()),
        creado,
    };
    Ok((elegidas, perfil))
}

/// Los addons instalados que el `AddOns.txt` del paquete deja activados (sin los del cliente): quien importa ve qué
/// le falta y dónde conseguirlo.
fn addons_del_paquete(piezas: &[Pieza], addons: &[Addon]) -> Vec<AddonPaquete> {
    let Some(lista) = piezas
        .iter()
        .find(|p| p.categoria == Categoria::AddonsActivos)
    else {
        return Vec::new();
    };
    let activos: HashSet<String> = lineas_addons(&lista.contenido)
        .filter(|(_, valor)| valor.eq_ignore_ascii_case(ACTIVADO))
        .map(|(carpeta, _)| carpeta.to_lowercase())
        .collect();
    addons
        .iter()
        .filter(|a| activos.contains(&a.carpeta.to_lowercase()))
        .filter_map(|a| {
            AddonPaquete::nuevo(
                &a.carpeta,
                a.datos.titulo.as_deref(),
                a.datos.version.as_deref(),
                a.datos.enlace.as_deref(),
                &a.datos.dependencias,
            )
        })
        .collect()
}

fn leer_pieza(
    personaje: &PersonajeUbicado,
    fuente: &Fuente,
    instalados: &HashSet<String>,
) -> Result<Option<Pieza>, ErrorNucleo> {
    for &ambito in fuente.origenes {
        let ruta = carpeta_de(personaje, ambito).join(fuente.fichero);
        if !ruta.is_file() {
            continue;
        }
        let bytes = std::fs::read(&ruta).map_err(|e| ErrorNucleo::lectura(&ruta, &e))?;
        let (contenido, claves_quitadas, tipo) = match fuente.filtro {
            Filtro::Ninguno => (bytes, Vec::new(), Tipo::Fichero),
            Filtro::Opciones => con_tipo(filtrar_opciones(&bytes)),
            Filtro::Addons => con_tipo(filtrar_addons(&bytes, instalados)),
        };
        return Ok(Some(Pieza {
            id: fuente.id,
            categoria: fuente.categoria,
            tipo,
            ambito,
            destino: fuente.destino,
            contenido,
            claves_quitadas,
        }));
    }
    Ok(None)
}

fn con_tipo((contenido, quitadas): (Vec<u8>, Vec<String>)) -> (Vec<u8>, Vec<String>, Tipo) {
    (contenido, quitadas, Tipo::FicheroFiltrado)
}

fn carpeta_de(personaje: &PersonajeUbicado, ambito: Ambito) -> PathBuf {
    match ambito {
        Ambito::Personaje => personaje.carpeta(),
        Ambito::Cuenta => cuenta_de(&personaje.raiz_version, &personaje.cuenta),
    }
}

fn cuenta_de(raiz_version: &Path, cuenta: &str) -> PathBuf {
    raiz_version
        .join(CARPETA_WTF)
        .join(CARPETA_ACCOUNT)
        .join(cuenta)
}

/// Quita las líneas enteras cuya clave decide `quitar`. Las que quedan conservan sus bytes, salto de línea incluido.
fn filtrar_lineas(
    bytes: &[u8],
    clave_de: impl Fn(&str) -> Option<String>,
    quitar: impl Fn(&str) -> bool,
) -> (Vec<u8>, Vec<String>) {
    let mut contenido = Vec::with_capacity(bytes.len());
    let mut quitadas = Vec::new();
    for linea in bytes.split_inclusive(|b| *b == b'\n') {
        match clave_de(&String::from_utf8_lossy(linea)) {
            Some(clave) if quitar(&clave) => quitadas.push(clave),
            _ => contenido.extend_from_slice(linea),
        }
    }
    (contenido, quitadas)
}

/// `SET <clave> "<valor>"`: la clave. Las demás líneas no se tocan.
fn clave_de_opcion(linea: &str) -> Option<String> {
    let mut partes = sin_bom(linea).split_whitespace();
    let set = partes.next()?;
    set.eq_ignore_ascii_case("SET")
        .then(|| partes.next())
        .flatten()
        .map(str::to_owned)
}

fn opcion_que_no_viaja(clave: &str) -> bool {
    let clave = clave.to_lowercase();
    CLAVES_EXACTAS.contains(&clave.as_str())
        || CLAVES_PREFIJO.iter().any(|p| clave.starts_with(p))
        || clave.contains(CLAVE_CONTIENE)
}

fn filtrar_opciones(bytes: &[u8]) -> (Vec<u8>, Vec<String>) {
    filtrar_lineas(bytes, clave_de_opcion, opcion_que_no_viaja)
}

fn filtrar_addons(bytes: &[u8], instalados: &HashSet<String>) -> (Vec<u8>, Vec<String>) {
    filtrar_lineas(
        bytes,
        |linea| addon_de_linea(linea).map(|(carpeta, _)| carpeta.to_owned()),
        |carpeta| {
            let carpeta = carpeta.to_lowercase();
            !carpeta.starts_with(PREFIJO_BLIZZARD) && !instalados.contains(&carpeta)
        },
    )
}

/// `<carpeta>: enabled|disabled` [V].
fn addon_de_linea(linea: &str) -> Option<(&str, &str)> {
    let (carpeta, valor) = sin_bom(linea).split_once(':')?;
    let carpeta = carpeta.trim();
    (!carpeta.is_empty()).then(|| (carpeta, valor.trim()))
}

fn lineas_addons(bytes: &[u8]) -> impl Iterator<Item = (String, String)> + '_ {
    bytes.split(|b| *b == b'\n').filter_map(|linea| {
        let texto = String::from_utf8_lossy(linea);
        addon_de_linea(&texto).map(|(c, v)| (c.to_owned(), v.to_owned()))
    })
}

/// Los ficheros medidos no traen BOM [V], pero si alguno lo trajera, la primera clave se tiene que reconocer igual.
fn sin_bom(linea: &str) -> &str {
    linea.trim_start_matches(BOM)
}

fn contiene(bytes: &[u8], buscado: &[u8]) -> bool {
    bytes.windows(buscado.len()).any(|v| v == buscado)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deteccion::Producto;
    use crate::toc::Toc;
    use std::fs::{create_dir_all, write};

    const CUENTA_PRUEBA: &str = "123#1";

    fn personaje(raiz: &Path) -> PersonajeUbicado {
        let ubicado = PersonajeUbicado {
            raiz_version: raiz.join("_forever_"),
            producto: Producto::Forever,
            cuenta: CUENTA_PRUEBA.into(),
            reino: "93".into(),
            personaje: "Goldberg-Joestar".into(),
            nombre_en_datos: None,
        };
        create_dir_all(ubicado.carpeta()).unwrap();
        ubicado
    }

    fn addon(carpeta: &str) -> Addon {
        Addon {
            carpeta: carpeta.into(),
            toc: format!("{carpeta}.toc"),
            datos: Toc {
                titulo: Some(format!("{carpeta} título")),
                enlace: Some("http://inseguro".into()),
                ..Toc::default()
            },
        }
    }

    fn ids(piezas: &[Pieza]) -> Vec<&str> {
        piezas.iter().map(|p| p.id).collect()
    }

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
                "hardTrackedQuests",
                "AllowDangerousScripts",
                "reputationsCollapsed",
                "TrackedAchievements",
                "cameraSavedPitch"
            ]
        );
    }

    #[test]
    fn un_bom_al_principio_no_esconde_la_primera_clave() {
        let opciones = "\u{feff}SET AllowDangerousScripts \"1\"\r\nSET a \"1\"\r\n";
        let (contenido, quitadas) = filtrar_opciones(opciones.as_bytes());
        assert_eq!(quitadas, ["AllowDangerousScripts"]);
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
        assert_eq!(quitadas, ["Carbonite", "AtlasLoot"]);
    }

    #[test]
    fn exporta_lo_que_hay_del_personaje_y_de_la_cuenta() {
        let tmp = tempfile::tempdir().unwrap();
        let pj = personaje(tmp.path());
        let cuenta = cuenta_de(&pj.raiz_version, CUENTA_PRUEBA);
        write(pj.carpeta().join("macros-cache.txt"), "VER 3\r\n").unwrap();
        write(
            pj.carpeta().join("config-cache.wtf"),
            "SET a \"1\"\r\nSET trackedX \"2\"\r\n",
        )
        .unwrap();
        write(
            pj.carpeta().join("AddOns.txt"),
            "Dominos: enabled\r\nViejo: enabled\r\n",
        )
        .unwrap();
        write(pj.carpeta().join("flagged-cache-character.txt"), "x").unwrap();
        write(cuenta.join("macros-cache.txt"), "VER 3\r\ngeneral\r\n").unwrap();
        write(cuenta.join("bindings-cache.wtf"), "bind Q X\r\n").unwrap();

        let (piezas, apartadas) = piezas(&pj, &[addon("Dominos")]).unwrap();

        assert_eq!(
            ids(&piezas),
            [
                "macros",
                "macros-cuenta",
                "teclas",
                "opciones",
                "addons-activos"
            ]
        );
        assert!(apartadas.is_empty());
        let teclas = &piezas[2];
        assert_eq!(
            (teclas.ambito, teclas.destino),
            (Ambito::Cuenta, "personaje/bindings-cache.wtf")
        );
        assert_eq!(piezas[3].contenido, b"SET a \"1\"\r\n");
        assert_eq!(piezas[3].tipo, Tipo::FicheroFiltrado);
        assert_eq!(piezas[4].claves_quitadas, ["Viejo"]);
        assert_eq!(piezas[0].tipo, Tipo::Fichero);
    }

    #[test]
    fn las_teclas_propias_ganan_a_las_de_la_cuenta() {
        let tmp = tempfile::tempdir().unwrap();
        let pj = personaje(tmp.path());
        write(pj.carpeta().join("bindings-cache.wtf"), "propias").unwrap();
        write(
            cuenta_de(&pj.raiz_version, CUENTA_PRUEBA).join("bindings-cache.wtf"),
            "cuenta",
        )
        .unwrap();
        let (piezas, _) = piezas(&pj, &[]).unwrap();
        assert_eq!(
            (piezas[0].ambito, piezas[0].contenido.as_slice()),
            (Ambito::Personaje, b"propias".as_slice())
        );
    }

    #[test]
    fn una_pieza_con_el_marcador_se_aparta() {
        let tmp = tempfile::tempdir().unwrap();
        let pj = personaje(tmp.path());
        write(
            pj.carpeta().join("macros-cache.txt"),
            "/say @@WOWPROFILE:PERSONAJE@@",
        )
        .unwrap();
        write(pj.carpeta().join("chat-cache.txt"), "WINDOW 1").unwrap();
        let (piezas, apartadas) = piezas(&pj, &[]).unwrap();
        assert_eq!(ids(&piezas), ["chat"]);
        assert_eq!(apartadas, ["macros"]);
    }

    #[test]
    fn elegir_se_queda_con_lo_pedido_y_valida_el_perfil() {
        let piezas = || {
            vec![
                crate::paquetes::tests::pieza("macros", b"a"),
                crate::paquetes::tests::pieza("chat", b"b"),
            ]
        };
        let pedido = |seleccion: &[&str], nombre: &str, descripcion: &str| Pedido {
            seleccion: seleccion.iter().map(|s| (*s).to_owned()).collect(),
            nombre: nombre.into(),
            descripcion: descripcion.into(),
        };
        let creado = || "2026-11-18T10:00:00Z".to_owned();

        let (elegidas, perfil) =
            elegir(piezas(), &pedido(&["chat"], "  Druida ", " "), creado()).unwrap();
        assert_eq!(ids(&elegidas), ["chat"]);
        assert_eq!(
            (perfil.nombre.as_str(), perfil.descripcion),
            ("Druida", None)
        );

        let largo = "x".repeat(MAX_NOMBRE + 1);
        for (seleccion, nombre, error) in [
            (&["chat"][..], "  ", ErrorNucleo::PerfilNoValido {}),
            (
                &["chat"][..],
                largo.as_str(),
                ErrorNucleo::PerfilNoValido {},
            ),
            (&[][..], "Druida", ErrorNucleo::SeleccionVacia {}),
            (&["otra"][..], "Druida", ErrorNucleo::SeleccionVacia {}),
        ] {
            assert_eq!(
                elegir(piezas(), &pedido(seleccion, nombre, ""), creado()).unwrap_err(),
                error
            );
        }
        let justo = "ñ".repeat(MAX_NOMBRE);
        assert!(elegir(piezas(), &pedido(&["chat"], &justo, ""), creado()).is_ok());
    }

    #[test]
    fn el_paquete_describe_los_addons_instalados_que_deja_activados() {
        let lista = Pieza {
            id: "addons-activos",
            categoria: Categoria::AddonsActivos,
            tipo: Tipo::FicheroFiltrado,
            ambito: Ambito::Personaje,
            destino: "personaje/AddOns.txt",
            contenido: b"Dominos: enabled\r\nDetails: disabled\r\nBlizzard_CombatText: enabled\r\n"
                .to_vec(),
            claves_quitadas: Vec::new(),
        };
        let addons = addons_del_paquete(&[lista], &[addon("Dominos"), addon("Details")]);
        assert_eq!(addons.len(), 1);
        assert_eq!(
            (addons[0].id.as_str(), addons[0].titulo.as_str()),
            ("Dominos", "Dominos título")
        );
        assert_eq!(addons[0].enlace, None, "solo https");
    }

    #[test]
    fn el_paquete_no_lleva_ni_cuenta_ni_reino_ni_personaje_ni_rutas() {
        let tmp = tempfile::tempdir().unwrap();
        let pj = personaje(tmp.path());
        write(
            tmp.path().join(".build.info"),
            "Version!STRING:0|Product!STRING:0\n1.60.1.70938|wowf\n",
        )
        .unwrap();
        write(
            pj.raiz_version.join("WTF").join("Config.wtf"),
            "SET engineSurveyPatch \"16001\"\r\n",
        )
        .unwrap();
        write(pj.carpeta().join("chat-cache.txt"), "WINDOW 1\r\n").unwrap();
        let pedido = Pedido {
            seleccion: vec!["chat".into()],
            nombre: "Druida".into(),
            descripcion: String::new(),
        };

        let paquete = paquete(&pj, &pedido).unwrap();

        let json = serde_json::to_string(&paquete.manifiesto).unwrap();
        // En el JSON, `\` va escapado.
        let ruta = tmp.path().to_string_lossy().replace('\\', "\\\\");
        for privado in [
            CUENTA_PRUEBA,
            "\"93\"",
            "Goldberg",
            "Joestar",
            ruta.as_str(),
        ] {
            assert!(!json.contains(privado), "{privado} en {json}");
        }
        assert_eq!(paquete.manifiesto.origen.interfaz, 16001);
        assert_eq!(paquete.manifiesto.entradas.len(), 1);
    }

    #[test]
    fn sin_interfaz_no_se_arma_el_paquete() {
        let tmp = tempfile::tempdir().unwrap();
        let pj = personaje(tmp.path());
        let pedido = Pedido {
            seleccion: vec!["chat".into()],
            nombre: "Druida".into(),
            descripcion: String::new(),
        };
        assert_eq!(
            paquete(&pj, &pedido).unwrap_err(),
            ErrorNucleo::ExportacionSinVersion {}
        );
    }

    #[test]
    fn el_nombre_de_fichero_no_lleva_lo_que_windows_no_admite() {
        assert_eq!(
            nombre_de_fichero(" Druida: feral/1 "),
            "Druida_ feral_1.wowprofile"
        );
        assert_eq!(nombre_de_fichero("Raid..."), "Raid.wowprofile");
        assert_eq!(nombre_de_fichero("con"), "con_.wowprofile");
        assert_eq!(nombre_de_fichero("COM1"), "COM1_.wowprofile");
        assert_eq!(nombre_de_fichero("Consola"), "Consola.wowprofile");
        assert_eq!(nombre_de_fichero("..."), "perfil.wowprofile");
    }

    #[test]
    fn la_extension_se_pone_si_falta() {
        assert_eq!(
            con_extension(PathBuf::from("a/b")),
            PathBuf::from("a/b.wowprofile")
        );
        assert_eq!(
            con_extension(PathBuf::from("a/b.WowProfile")),
            PathBuf::from("a/b.WowProfile")
        );
        assert_eq!(
            con_extension(PathBuf::from("a/b.txt")),
            PathBuf::from("a/b.txt.wowprofile")
        );
    }
}
