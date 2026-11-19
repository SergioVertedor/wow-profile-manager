//! Exportación (`docs/arquitectura.md`): arma las piezas de un personaje para un `.wowprofile`: los ficheros que se
//! copian sin leer Lua (spec 023) y la configuración de addons (spec 027). Solo lee; el paquete lo escribe Paquetes.

use crate::addons::Addon;
use crate::claves_personaje::claves_de_personaje;
use crate::config_addons::{self, ConfigAddons, NoExportable, Pista, Regla};
use crate::descubrimiento::{CARPETA_SAVED_VARIABLES as CARPETA_SV, nombres_de_carpetas};
use crate::deteccion::{self, PersonajeUbicado};
use crate::error::ErrorNucleo;
use crate::modo_edicion;
use crate::operaciones::{ahora_rfc3339, segmento_literal};
use crate::paquetes::{
    AddonPaquete, Ambito, Categoria, DeAddon, MARCADOR, Origen, Paquete, Perfil, Pieza,
    RefAdaptador, Tipo,
};
use crate::piezas::{
    ACTIVADO, FUENTES, Filtro, Fuente, Quitadas, filtrar_addons, filtrar_opciones, lineas_addons,
};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

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
    /// Lo que no se puede exportar de cada addon, con su motivo.
    pub no_exportables: Vec<NoExportable>,
    pub juego_abierto: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiezaPropuesta {
    pub id: String,
    pub categoria: Categoria,
    pub tipo: Tipo,
    pub ambito: Ambito,
    pub bytes: u64,
    /// Líneas que se quitan al filtrar, por motivo.
    pub quitadas: Vec<Quitadas>,
    /// Si sale marcada en el diálogo (`docs/flujos.md`, «Exportar», paso 3).
    pub marcada: bool,
    /// Solo en `config-addon`.
    pub addon: Option<String>,
    pub variable: Option<String>,
    /// Nombre del perfil AceDB tal como lo ve quien exporta.
    pub perfil: Option<String>,
    pub regla: Option<Regla>,
    pub version_no_medida: bool,
    pub pistas: Vec<Pista>,
}

/// Todo lo que se puede llevar del personaje ahora mismo.
struct Disponible {
    /// Ficheros de la entrega 1, sin las apartadas.
    ficheros: Vec<Pieza>,
    apartadas: Vec<String>,
    config: ConfigAddons,
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
    Ok(propuesta(
        disponible(personaje, &addons.addons)?,
        juego_abierto,
    ))
}

/// El paquete con lo pedido, leído del disco en este momento (lo que se guarda es lo que hay al guardar).
pub fn paquete(personaje: &PersonajeUbicado, pedido: &Pedido) -> Result<Paquete, ErrorNucleo> {
    let (Some(build), Some(interfaz)) =
        deteccion::origen_de(&personaje.raiz_version, personaje.producto)?
    else {
        return Err(ErrorNucleo::ExportacionSinVersion {});
    };
    let addons = deteccion::addons_de(&personaje.raiz_version, personaje.producto)?;
    let disponible = disponible(personaje, &addons.addons)?;
    let todas = disponible
        .ficheros
        .into_iter()
        .chain(disponible.config.piezas.into_iter().map(|p| p.pieza))
        .collect();
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

fn disponible(personaje: &PersonajeUbicado, addons: &[Addon]) -> Result<Disponible, ErrorNucleo> {
    let (ficheros, apartadas) = piezas(personaje, addons)?;
    let config = config_addons::de_personaje(personaje, addons, &claves_de_cuenta(personaje)?);
    Ok(Disponible {
        ficheros,
        apartadas,
        config,
    })
}

/// Cómo se llaman en los datos los personajes de su cuenta: los de `profileKeys` y los de las carpetas.
fn claves_de_cuenta(personaje: &PersonajeUbicado) -> Result<HashSet<String>, ErrorNucleo> {
    let cuenta = personaje.carpeta_cuenta();
    let mut claves = claves_de_personaje(&cuenta.join(CARPETA_SV))?;
    claves.extend(nombres_de_carpetas(&cuenta, personaje.producto)?);
    Ok(claves)
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
            apartadas.push(pieza.id.clone());
        } else {
            piezas.push(pieza);
        }
    }
    Ok((piezas, apartadas))
}

fn propuesta(disponible: Disponible, juego_abierto: bool) -> Propuesta {
    let ficheros = disponible.ficheros.iter().map(|p| PiezaPropuesta {
        marcada: true,
        ..propuesta_de_pieza(p)
    });
    let config = disponible.config.piezas.iter().map(|p| {
        let de_addon = p.pieza.addon.as_ref();
        PiezaPropuesta {
            marcada: p.marcada,
            addon: de_addon.map(|d| d.addon.clone()),
            variable: de_addon.and_then(|d| d.variable.clone()),
            perfil: p.perfil.clone(),
            regla: Some(p.regla),
            version_no_medida: p.version_no_medida,
            pistas: p.pistas.clone(),
            ..propuesta_de_pieza(&p.pieza)
        }
    });
    Propuesta {
        piezas: ficheros.chain(config).collect(),
        apartadas: disponible.apartadas,
        no_exportables: disponible.config.no_exportables,
        juego_abierto,
    }
}

fn propuesta_de_pieza(pieza: &Pieza) -> PiezaPropuesta {
    PiezaPropuesta {
        id: pieza.id.clone(),
        categoria: pieza.categoria,
        tipo: pieza.tipo,
        ambito: pieza.ambito,
        bytes: pieza.contenido.len() as u64,
        quitadas: pieza.quitadas.clone(),
        marcada: false,
        addon: None,
        variable: None,
        // El Modo Edición lleva aquí el nombre del diseño.
        perfil: pieza.addon.as_ref().and_then(|d| d.perfil.clone()),
        regla: None,
        version_no_medida: false,
        pistas: Vec::new(),
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
        .filter(|p| pedido.seleccion.contains(&p.id))
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

/// Los addons instalados que el `AddOns.txt` del paquete deja activados (sin los del cliente) y los de las piezas
/// `config-addon`: quien importa ve qué le falta y dónde conseguirlo.
fn addons_del_paquete(piezas: &[Pieza], addons: &[Addon]) -> Vec<AddonPaquete> {
    let mut activos: HashSet<String> = piezas
        .iter()
        .filter(|p| p.categoria == Categoria::AddonsActivos)
        .flat_map(|lista| {
            lineas_addons(&lista.contenido)
                .filter(|(_, valor)| valor.eq_ignore_ascii_case(ACTIVADO))
                .map(|(carpeta, _)| carpeta.to_lowercase())
                .collect::<Vec<_>>()
        })
        .collect();
    activos.extend(
        piezas
            .iter()
            .filter_map(|p| p.addon.as_ref())
            .map(|d| d.addon.to_lowercase()),
    );
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
        let (contenido, quitadas, tipo) = match fuente.filtro {
            Filtro::Ninguno => (bytes, Vec::new(), Tipo::Fichero),
            Filtro::Opciones => con_tipo(filtrar_opciones(&bytes)),
            Filtro::Addons => con_tipo(filtrar_addons(&bytes, instalados)),
            Filtro::ModoEdicion => return pieza_modo_edicion(personaje, fuente, ambito, &bytes),
        };
        return Ok(Some(Pieza {
            id: fuente.id.to_owned(),
            categoria: fuente.categoria,
            tipo,
            ambito,
            destino: fuente.destino.to_owned(),
            contenido,
            quitadas,
            addon: None,
        }));
    }
    Ok(None)
}

/// El diseño del Modo Edición que usa el personaje, si es uno propio. Si los ficheros no se entienden no se exporta
/// (queda en el registro de diagnóstico): mejor no llevarlo que llevar algo roto.
fn pieza_modo_edicion(
    personaje: &PersonajeUbicado,
    fuente: &Fuente,
    ambito: Ambito,
    cuenta: &[u8],
) -> Result<Option<Pieza>, ErrorNucleo> {
    let ruta = personaje.carpeta().join(modo_edicion::FICHERO_PERSONAJE);
    if !ruta.is_file() {
        return Ok(None);
    }
    let del_personaje = std::fs::read(&ruta).map_err(|e| ErrorNucleo::lectura(&ruta, &e))?;
    let (nombre, contenido) = match modo_edicion::exportar(cuenta, &del_personaje) {
        Ok(Some(diseno)) => diseno,
        Ok(None) => return Ok(None),
        Err(motivo) => {
            log::warn!("Modo Edición de {} no exportado: {motivo}", ruta.display());
            return Ok(None);
        }
    };
    Ok(Some(Pieza {
        id: fuente.id.to_owned(),
        categoria: fuente.categoria,
        tipo: Tipo::Adaptador,
        ambito,
        destino: fuente.destino.to_owned(),
        contenido,
        quitadas: Vec::new(),
        addon: Some(DeAddon {
            perfil: Some(nombre),
            adaptador: Some(RefAdaptador {
                id: modo_edicion::ADAPTADOR.to_owned(),
                version: modo_edicion::VERSION_ADAPTADOR,
            }),
            ..DeAddon::default()
        }),
    }))
}

fn con_tipo((contenido, quitadas): (Vec<u8>, Vec<Quitadas>)) -> (Vec<u8>, Vec<Quitadas>, Tipo) {
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
    use crate::piezas::Motivo;
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
        piezas.iter().map(|p| p.id.as_str()).collect()
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
            (teclas.ambito, teclas.destino.as_str()),
            (Ambito::Cuenta, "personaje/bindings-cache.wtf")
        );
        assert_eq!(piezas[3].contenido, b"SET a \"1\"\r\n");
        assert_eq!(piezas[3].tipo, Tipo::FicheroFiltrado);
        assert_eq!(
            piezas[4].quitadas,
            [Quitadas {
                motivo: Motivo::AddonNoInstalado,
                claves: vec!["Viejo".to_owned()],
            }]
        );
        assert_eq!(piezas[0].tipo, Tipo::Fichero);
    }

    /// Cuenta con dos diseños del Modo Edición (`default` y `Sanador`), con la forma medida.
    const DISENOS: &str = "5 1 0 7 default 0 1 0 0 1 7 7 UIParent 0.0 -4.0 -1 ## 7 Sanador 0 1 0 0 1 7 7 UIParent 1.0 2.0 -1 #$\0";

    #[test]
    fn del_modo_edicion_viaja_solo_el_diseno_que_usa_el_personaje() {
        let tmp = tempfile::tempdir().unwrap();
        let pj = personaje(tmp.path());
        write(
            pj.carpeta_cuenta().join("edit-mode-cache-account.txt"),
            DISENOS,
        )
        .unwrap();
        write(
            pj.carpeta().join("edit-mode-cache-character.txt"),
            "5 4 3 3 3 3 -1 -1 -1 -1 -1\0",
        )
        .unwrap();

        let (piezas, _) = piezas(&pj, &[]).unwrap();

        let pieza = piezas.iter().find(|p| p.id == "modo-edicion").unwrap();
        assert_eq!(
            (pieza.tipo, pieza.ambito, pieza.destino.as_str()),
            (
                Tipo::Adaptador,
                Ambito::Cuenta,
                "cuenta/edit-mode-cache-account.txt"
            )
        );
        assert_eq!(
            pieza.contenido,
            b"5 1 0 7 Sanador 0 1 0 0 1 7 7 UIParent 1.0 2.0 -1 #$\0"
        );
        let de_addon = pieza.addon.as_ref().unwrap();
        assert_eq!(de_addon.perfil.as_deref(), Some("Sanador"));
        assert_eq!(de_addon.adaptador.as_ref().unwrap().id, "modo-edicion");
        assert_eq!(propuesta_de_pieza(pieza).perfil.as_deref(), Some("Sanador"));
    }

    #[test]
    fn con_un_diseno_de_blizzard_o_ficheros_raros_no_hay_modo_edicion() {
        for del_personaje in ["5 0 3 3 3 3 -1 -1 -1 -1 -1\0", "roto"] {
            let tmp = tempfile::tempdir().unwrap();
            let pj = personaje(tmp.path());
            write(
                pj.carpeta_cuenta().join("edit-mode-cache-account.txt"),
                DISENOS,
            )
            .unwrap();
            write(
                pj.carpeta().join("edit-mode-cache-character.txt"),
                del_personaje,
            )
            .unwrap();

            let (piezas, _) = piezas(&pj, &[]).unwrap();

            assert!(!ids(&piezas).contains(&"modo-edicion"), "{del_personaje}");
        }
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
            id: "addons-activos".into(),
            categoria: Categoria::AddonsActivos,
            tipo: Tipo::FicheroFiltrado,
            ambito: Ambito::Personaje,
            destino: "personaje/AddOns.txt".into(),
            contenido: b"Dominos: enabled\r\nDetails: disabled\r\nBlizzard_CombatText: enabled\r\n"
                .to_vec(),
            quitadas: Vec::new(),
            addon: None,
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
