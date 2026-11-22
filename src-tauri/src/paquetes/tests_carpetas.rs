//! Paquetes con carpetas de addons (formato 1.1, #73): ida y vuelta, y lo que la lectura rechaza.

use super::tests::{entradas_zip, pieza};
use super::*;
use crate::error::MotivoPaquete;
use serde_json::{Value, json};
use std::fs::{create_dir_all, write};
use std::io::Cursor;

fn perfil() -> Perfil {
    Perfil {
        nombre: "Druida".into(),
        descripcion: None,
        creado: "2026-11-19T10:00:00Z".into(),
    }
}

fn origen() -> Origen {
    Origen {
        producto: "wowf",
        build: "1.60.1.70938".into(),
        interfaz: 16001,
    }
}

/// Un addon `Dominos` de prueba en `raiz` y su pieza.
fn carpeta(raiz: &Path) -> CarpetaAddon {
    let origen = raiz.join("Dominos");
    create_dir_all(origen.join("Media")).unwrap();
    write(origen.join("Dominos.toc"), "## Title: Dominos").unwrap();
    write(origen.join("Media/barra.tga"), [7u8; 100]).unwrap();
    CarpetaAddon {
        id: "carpeta-dominos".into(),
        carpeta: "Dominos".into(),
        origen,
    }
}

/// Guarda un paquete con macros y la carpeta de `Dominos`, y devuelve su ruta.
fn guardado(tmp: &Path) -> PathBuf {
    let ruta = tmp.join("p.wowprofile");
    Paquete::nuevo(
        perfil(),
        origen(),
        Vec::new(),
        vec![pieza("macros", b"VER 3\r\n")],
    )
    .unwrap()
    .con_carpetas(vec![carpeta(tmp)])
    .unwrap()
    .guardar(&ruta)
    .unwrap();
    ruta
}

/// Un ZIP con estos ficheros, en este orden.
fn zip_de(ficheros: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (nombre, bytes) in ficheros {
        zip.start_file(nombre.as_str(), SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

/// El paquete guardado, con el manifiesto cambiado por `cambiar` y la pieza de la carpeta cambiada por `pieza` (si
/// viene, con su hash y su tamaño al día). Se lee como fichero.
fn retocado(
    tmp: &Path,
    cambiar: impl FnOnce(&mut Value),
    pieza: Option<Vec<u8>>,
) -> Result<PaqueteLeido, ErrorNucleo> {
    let mut ficheros = entradas_zip(&std::fs::read(guardado(tmp)).unwrap());
    let indice = |nombre: &str, ficheros: &[(String, Vec<u8>)]| {
        ficheros.iter().position(|(n, _)| n == nombre).unwrap()
    };
    let i_manifiesto = indice(MANIFIESTO, &ficheros);
    let mut manifiesto: Value = serde_json::from_slice(&ficheros[i_manifiesto].1).unwrap();
    if let Some(bytes) = pieza {
        let entrada = &mut manifiesto["entradas"][1];
        entrada["sha256"] = json!(sha256(&bytes));
        entrada["bytes"] = json!(bytes.len());
        let i_pieza = indice("datos/carpeta-dominos", &ficheros);
        ficheros[i_pieza].1 = bytes;
    }
    cambiar(&mut manifiesto);
    ficheros[i_manifiesto].1 = serde_json::to_vec(&manifiesto).unwrap();
    let ruta = tmp.join("retocado.wowprofile");
    write(&ruta, zip_de(&ficheros)).unwrap();
    leer_fichero(&ruta)
}

fn motivo(leido: Result<PaqueteLeido, ErrorNucleo>) -> MotivoPaquete {
    match leido {
        Err(ErrorNucleo::PaqueteInvalido { motivo, .. }) => motivo,
        otro => panic!("se esperaba un paquete inválido: {otro:?}"),
    }
}

#[test]
fn una_carpeta_viaja_como_pieza_1_1_y_no_se_queda_en_memoria() {
    let tmp = tempfile::tempdir().unwrap();
    let ruta = guardado(tmp.path());

    let leido = leer_fichero(&ruta).unwrap();

    assert_eq!(leido.manifiesto.formato.version, "1.1");
    let entrada = &leido.manifiesto.entradas[1];
    assert_eq!(
        (
            entrada.categoria.as_str(),
            entrada.tipo.as_str(),
            entrada.ambito,
            entrada.destino.as_str(),
            entrada.addon.as_deref()
        ),
        (
            "addon",
            "carpeta",
            AmbitoEntrada::Instalacion,
            "addons/Dominos",
            Some("Dominos")
        )
    );
    assert_eq!(entrada.categoria_conocida(), Some(Categoria::Addon));
    assert_eq!(leido.pieza("macros"), Some(b"VER 3\r\n".as_slice()));
    assert_eq!(leido.pieza("carpeta-dominos"), None);
    assert_eq!(leido.sha256, sha256(&std::fs::read(&ruta).unwrap()));
    // La pieza es el ZIP del addon tal cual se empaqueta.
    let ficheros = entradas_zip(&std::fs::read(&ruta).unwrap());
    let (_, pieza) = ficheros
        .iter()
        .find(|(n, _)| n == "datos/carpeta-dominos")
        .unwrap();
    let dentro = addon_zip::leer(pieza).unwrap();
    assert_eq!(dentro.len(), 2);
}

#[test]
fn sin_carpetas_el_paquete_sigue_siendo_1_0() {
    let paquete = Paquete::nuevo(perfil(), origen(), Vec::new(), vec![pieza("macros", b"x")])
        .unwrap()
        .con_carpetas(Vec::new())
        .unwrap();
    assert_eq!(paquete.manifiesto.formato.version, "1.0");
}

#[test]
fn una_carpeta_que_no_se_puede_empaquetar_no_deja_paquete() {
    let tmp = tempfile::tempdir().unwrap();
    let mala = carpeta(tmp.path());
    write(mala.origen.join("CON.lua"), "x").unwrap();
    let ruta = tmp.path().join("p.wowprofile");

    let error = Paquete::nuevo(perfil(), origen(), Vec::new(), Vec::new())
        .unwrap()
        .con_carpetas(vec![mala])
        .unwrap()
        .guardar(&ruta)
        .unwrap_err();

    assert!(
        matches!(&error, ErrorNucleo::AddonNoEmpaquetado { addon, .. } if addon == "Dominos"),
        "{error:?}"
    );
    assert!(!ruta.exists());
    assert!(!tmp.path().join("p.wowprofile.tmp").exists());
}

#[test]
fn una_entrada_de_addon_mal_descrita_no_cumple_el_schema() {
    let casos: [(&str, Value); 6] = [
        ("destino", json!("addons/Otro")),
        ("destino", json!("cuenta/SavedVariables/Dominos.lua")),
        ("ambito", json!("cuenta")),
        ("tipo", json!("fichero")),
        ("bytes", json!(addon_zip::MAX_BYTES_ADDON + 1)),
        ("perfil", json!("Default")),
    ];
    for (campo, valor) in casos {
        let tmp = tempfile::tempdir().unwrap();
        let leido = retocado(
            tmp.path(),
            |m| m["entradas"][1][campo] = valor.clone(),
            None,
        );
        assert_eq!(motivo(leido), MotivoPaquete::ManifiestoNoValido, "{campo}");
    }
}

#[test]
fn un_addon_con_nombre_de_dispositivo_o_punto_final_no_se_acepta() {
    for addon in ["CON", "nul", "COM1", "Dominos."] {
        let tmp = tempfile::tempdir().unwrap();
        let leido = retocado(
            tmp.path(),
            |m| {
                m["entradas"][1]["addon"] = json!(addon);
                m["entradas"][1]["destino"] = json!(format!("addons/{addon}"));
            },
            None,
        );
        assert_eq!(motivo(leido), MotivoPaquete::ManifiestoNoValido, "{addon}");
    }
}

#[test]
fn una_carpeta_con_id_repetido_o_no_valido_no_entra_en_el_paquete() {
    let tmp = tempfile::tempdir().unwrap();
    let buena = carpeta(tmp.path());
    let casos = [
        CarpetaAddon {
            id: "macros".into(),
            ..buena.clone()
        },
        CarpetaAddon {
            carpeta: "CON".into(),
            ..buena.clone()
        },
        CarpetaAddon {
            id: "Con Mayusculas".into(),
            ..buena
        },
    ];
    for mala in casos {
        let error = Paquete::nuevo(perfil(), origen(), Vec::new(), vec![pieza("macros", b"x")])
            .unwrap()
            .con_carpetas(vec![mala.clone()])
            .unwrap_err();
        assert!(
            matches!(error, ErrorNucleo::AddonNoEmpaquetado { .. }),
            "{mala:?}"
        );
    }
}

#[test]
fn una_pieza_de_configuracion_no_puede_ser_de_la_instalacion() {
    let tmp = tempfile::tempdir().unwrap();
    let leido = retocado(
        tmp.path(),
        |m| m["entradas"][0]["ambito"] = json!("instalacion"),
        None,
    );
    assert_eq!(motivo(leido), MotivoPaquete::ManifiestoNoValido);
    let leido = retocado(
        tmp.path(),
        |m| m["entradas"][0]["tipo"] = json!("carpeta"),
        None,
    );
    assert_eq!(motivo(leido), MotivoPaquete::ManifiestoNoValido);
}

#[test]
fn un_addon_que_saldria_de_su_carpeta_rechaza_el_paquete_entero() {
    for malo in ["../Interface.lua", "C:/Windows/x.lua", "CON.lua"] {
        let tmp = tempfile::tempdir().unwrap();
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(malo, SimpleFileOptions::default()).unwrap();
        zip.write_all(b"print('hola')").unwrap();
        let pieza = zip.finish().unwrap().into_inner();

        let leido = retocado(tmp.path(), |_| {}, Some(pieza));

        assert_eq!(motivo(leido), MotivoPaquete::AddonNoValido, "{malo}");
    }
}

#[test]
fn una_pieza_de_addon_cambiada_no_cuadra_con_su_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let leido = retocado(
        tmp.path(),
        |m| m["entradas"][1]["sha256"] = json!("0".repeat(64)),
        None,
    );
    assert_eq!(motivo(leido), MotivoPaquete::PiezaDanada);
}
