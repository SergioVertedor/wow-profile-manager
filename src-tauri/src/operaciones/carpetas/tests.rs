use super::super::tests::{Escenario, diario_de, escenario};
use super::super::{Estado, PaqueteImportado};
use super::*;
use std::cell::Cell;
use std::fs::{create_dir_all, read_to_string, write};

fn paquete() -> PaqueteImportado {
    PaqueteImportado {
        nombre: "Goldberg".into(),
        sha256: "a".repeat(64),
    }
}

fn ficheros() -> Vec<FicheroAddon> {
    vec![
        FicheroAddon {
            ruta: "Dominos.toc".into(),
            contenido: b"## Title: Dominos".to_vec(),
        },
        FicheroAddon {
            ruta: "Media/barra.tga".into(),
            contenido: vec![7; 10],
        },
    ]
}

fn dominos() -> Vec<InstalarCarpeta> {
    vec![InstalarCarpeta {
        carpeta: "Dominos".into(),
        entrada: "carpeta-dominos".into(),
    }]
}

fn cargar(_: &str) -> Result<Vec<FicheroAddon>, ErrorNucleo> {
    Ok(ficheros())
}

fn addons(e: &Escenario) -> PathBuf {
    crate::deteccion::carpeta_addons(&e.personaje.raiz_version)
}

/// Lo que hay en `Interface/AddOns`, por nombre.
fn en_addons(e: &Escenario) -> Vec<String> {
    let mut nombres: Vec<String> = std::fs::read_dir(addons(e))
        .map(|d| {
            d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    nombres.sort();
    nombres
}

fn instalar(e: &Escenario) -> String {
    e.operaciones
        .importar(
            &e.personaje,
            paquete(),
            Vec::new(),
            dominos(),
            &cargar,
            || false,
        )
        .unwrap()
}

#[test]
fn una_importacion_instala_la_carpeta_entera_y_lo_apunta_en_el_diario() {
    let e = escenario();

    let id = instalar(&e);

    let dominos = addons(&e).join("Dominos");
    assert_eq!(
        read_to_string(dominos.join("Dominos.toc")).unwrap(),
        "## Title: Dominos"
    );
    assert_eq!(
        std::fs::read(dominos.join("Media/barra.tga")).unwrap(),
        [7; 10]
    );
    assert_eq!(en_addons(&e), ["Dominos"], "sin temporales");
    let diario = diario_de(&e.datos, &id);
    assert_eq!(diario.estado, Estado::Completa);
    assert!(diario.cambios.is_empty());
    assert_eq!(diario.carpetas[0].ruta, "Interface/AddOns/Dominos");
    assert_eq!(
        diario.carpetas[0].ficheros[1],
        FicheroInstalado {
            ruta: "Media/barra.tga".into(),
            sha256: sha256(&[7; 10]),
        }
    );
}

#[test]
fn una_carpeta_que_ya_existe_aunque_sea_con_otras_mayusculas_no_se_pisa() {
    let e = escenario();
    create_dir_all(addons(&e).join("DOMINOS")).unwrap();
    write(addons(&e).join("DOMINOS/mio.lua"), "mio").unwrap();

    let resultado = e.operaciones.importar(
        &e.personaje,
        paquete(),
        Vec::new(),
        dominos(),
        &cargar,
        || false,
    );

    assert_eq!(resultado, Err(ErrorNucleo::PlanCaducado {}));
    assert_eq!(en_addons(&e), ["DOMINOS"]);
    assert_eq!(
        read_to_string(addons(&e).join("DOMINOS/mio.lua")).unwrap(),
        "mio"
    );
}

#[test]
fn si_la_pieza_cambia_entre_preparar_e_instalar_no_queda_nada_a_medias() {
    let e = escenario();
    let veces = Cell::new(0);
    let cambia = |_: &str| {
        veces.set(veces.get() + 1);
        let mut ficheros = ficheros();
        if veces.get() > 1 {
            ficheros[1].contenido = vec![0; 10];
        }
        Ok(ficheros)
    };

    let resultado = e.operaciones.importar(
        &e.personaje,
        paquete(),
        Vec::new(),
        dominos(),
        &cambia,
        || false,
    );

    let Err(ErrorNucleo::OperacionFallida { restaurada, .. }) = resultado else {
        panic!("{resultado:?}");
    };
    assert!(restaurada);
    assert!(en_addons(&e).is_empty(), "{:?}", en_addons(&e));
}

#[test]
fn deshacer_quita_lo_instalado_y_deja_lo_que_cambio_despues() {
    let e = escenario();
    let id = instalar(&e);
    let dominos = addons(&e).join("Dominos");
    write(dominos.join("Dominos.toc"), "## Title: Dominos 2").unwrap();

    let plan = e.operaciones.plan_deshacer(&id).unwrap();

    assert_eq!(plan.addons, ["Interface/AddOns/Dominos"]);
    assert_eq!(
        plan.addons_cambiados,
        [dominos.join("Dominos.toc").display().to_string()]
    );
    e.operaciones.deshacer(&id, &plan.huella, || false).unwrap();
    assert_eq!(
        read_to_string(dominos.join("Dominos.toc")).unwrap(),
        "## Title: Dominos 2"
    );
    assert!(!dominos.join("Media").exists(), "carpeta vacía quitada");
    assert_eq!(diario_de(&e.datos, &id).estado, Estado::Deshecha);
}

#[test]
fn deshacer_sin_cambios_quita_la_carpeta_entera() {
    let e = escenario();
    let id = instalar(&e);
    let plan = e.operaciones.plan_deshacer(&id).unwrap();
    assert!(plan.addons_cambiados.is_empty());

    e.operaciones.deshacer(&id, &plan.huella, || false).unwrap();

    assert!(en_addons(&e).is_empty());
}

#[test]
fn una_ruta_del_diario_fuera_de_addons_no_se_usa() {
    let tmp = tempfile::tempdir().unwrap();
    for mala in [
        "Interface/AddOns/../../WTF",
        "Interface/AddOns/CON",
        "Interface/AddOns/a/b",
        "WTF/Account/x",
        "Interface/AddOns/",
    ] {
        assert!(en_disco(tmp.path(), mala).is_err(), "{mala}");
    }
    let carpeta = CarpetaInstalada {
        ruta: "Interface/AddOns/Dominos".into(),
        entrada: "x".into(),
        ficheros: vec![FicheroInstalado {
            ruta: "../../../fuera.txt".into(),
            sha256: sha256(b"x"),
        }],
    };
    let fuera = tmp.path().join("fuera.txt");
    write(&fuera, "x").unwrap();
    create_dir_all(tmp.path().join("Interface/AddOns/Dominos")).unwrap();

    let quedan = quitar(tmp.path(), "op", &[carpeta]);

    assert!(fuera.exists());
    assert_eq!(quedan.len(), 1);
}

#[test]
fn un_fichero_anadido_despues_se_avisa_y_se_queda() {
    let e = escenario();
    let id = instalar(&e);
    let extra = addons(&e).join("Dominos").join("Media").join("mio.tga");
    write(&extra, "mio").unwrap();

    let plan = e.operaciones.plan_deshacer(&id).unwrap();

    assert_eq!(plan.addons_cambiados, [extra.display().to_string()]);
    e.operaciones.deshacer(&id, &plan.huella, || false).unwrap();
    assert_eq!(read_to_string(&extra).unwrap(), "mio");
    assert!(!addons(&e).join("Dominos/Dominos.toc").exists());
}

/// Medido en Windows: renombrar encima de una carpeta vacía la sustituye. Si aparece una entre preparar e instalar, no
/// se instala.
#[test]
fn una_carpeta_vacia_que_aparece_antes_de_renombrar_no_se_sustituye() {
    let e = escenario();
    let vacia = addons(&e).join("dominos");
    let veces = Cell::new(0);
    let aparece = |_: &str| {
        veces.set(veces.get() + 1);
        if veces.get() == 2 {
            create_dir_all(&vacia).unwrap();
        }
        Ok(ficheros())
    };

    let resultado = e.operaciones.importar(
        &e.personaje,
        paquete(),
        Vec::new(),
        dominos(),
        &aparece,
        || false,
    );

    assert!(matches!(
        resultado,
        Err(ErrorNucleo::OperacionFallida {
            restaurada: true,
            ..
        })
    ));
    assert_eq!(en_addons(&e), ["dominos"]);
}

/// Una unión de Windows (`mklink /J`) puesta dentro del addon instalado no se sigue al quitarlo.
#[cfg(windows)]
#[test]
fn quitar_no_sigue_una_union_dentro_del_addon() {
    let e = escenario();
    let id = instalar(&e);
    let media = addons(&e).join("Dominos").join("Media");
    let fuera = e.personaje.raiz_version.join("fuera");
    create_dir_all(&fuera).unwrap();
    write(fuera.join("barra.tga"), [7u8; 10]).unwrap();
    std::fs::remove_dir_all(&media).unwrap();
    let creada = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&media)
        .arg(&fuera)
        .output()
        .unwrap();
    assert!(creada.status.success(), "{creada:?}");

    let plan = e.operaciones.plan_deshacer(&id).unwrap();
    e.operaciones.deshacer(&id, &plan.huella, || false).unwrap();

    assert!(fuera.join("barra.tga").exists(), "lo de fuera no se toca");
}

#[test]
fn una_importacion_posterior_del_mismo_addon_impide_deshacer_la_primera() {
    let e = escenario();
    let primera = instalar(&e);
    std::fs::remove_dir_all(addons(&e).join("Dominos")).unwrap();
    let segunda = instalar(&e);

    assert_eq!(
        e.operaciones.plan_deshacer(&primera),
        Err(ErrorNucleo::OperacionPosterior { id: segunda })
    );
}
