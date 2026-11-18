//! Exportación (`docs/arquitectura.md`): arma las piezas de un personaje para un `.wowprofile` (spec 023, entrega 1 de
//! #56: los ficheros que se copian sin leer Lua). Solo lee; el paquete lo escribe Paquetes.

use crate::addons::Addon;
use crate::deteccion::{self, PersonajeUbicado};
use crate::error::ErrorNucleo;
use crate::operaciones::{ahora_rfc3339, segmento_literal};
use crate::paquetes::{AddonPaquete, Ambito, Categoria, Origen, Paquete, Perfil, Pieza, Tipo};
use crate::piezas::{
    ACTIVADO, FUENTES, Filtro, Fuente, filtrar_addons, filtrar_opciones, lineas_addons,
};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

/// Texto que el formato reserva para sus marcadores: una pieza que ya lo trae no viaja.
const MARCADOR: &[u8] = b"@@WOWPROFILE:";
const MAX_NOMBRE: usize = 80;
const MAX_DESCRIPCION: usize = 500;
pub const EXTENSION: &str = "wowprofile";
/// Caracteres que Windows no admite en un nombre de fichero.
const NO_ADMITIDOS_EN_FICHERO: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
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
        Ambito::Cuenta => personaje.carpeta_cuenta(),
    }
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
    use std::path::Path;

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
    fn exporta_lo_que_hay_del_personaje_y_de_la_cuenta() {
        let tmp = tempfile::tempdir().unwrap();
        let pj = personaje(tmp.path());
        let cuenta = pj.carpeta_cuenta();
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
        write(pj.carpeta_cuenta().join("bindings-cache.wtf"), "cuenta").unwrap();
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
