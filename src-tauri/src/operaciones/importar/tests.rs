use super::super::tests::{ADDONS_TXT, Escenario, PRAT_LUA, diario_de, escenario};
use super::super::{DIARIO, Recuperacion, ficheros_de, subcarpetas};
use super::*;
use std::fs::{create_dir_all, read, read_to_string, write};
use std::path::{Path, PathBuf};

const MACROS: &str =
    "VER 3 0100000000000001 \" \" \"INV_Misc_QuestionMark\"\r\n/say hola\r\nEND\r\n";
const ADDONS_NUEVO: &str = "Prat-3.0: disabled\r\n";
const CONFIG_CUENTA: &str = "SET autoLootDefault \"1\"\r\n";
const CONFIG_VIEJA: &str = "SET a \"0\"\r\n";

fn paquete() -> PaqueteImportado {
    PaqueteImportado {
        nombre: "Goldberg".into(),
        sha256: "a".repeat(64),
    }
}

fn escritura(fichero: &str, de_cuenta: bool, contenido: &str, antes: Option<&str>) -> Escritura {
    Escritura {
        fichero: fichero.into(),
        de_cuenta,
        entrada: "e1".into(),
        accion: match antes {
            None => AccionImportar::Crear,
            Some(_) => AccionImportar::Fusionar,
        },
        contenido: contenido.as_bytes().to_vec(),
        antes_sha256: antes.map(|a| sha256(a.as_bytes())),
    }
}

/// Macros nuevas en el personaje, `AddOns.txt` fusionado y opciones de la cuenta nuevas.
fn tres() -> Vec<Escritura> {
    vec![
        escritura("macros-cache.txt", false, MACROS, None),
        escritura("AddOns.txt", false, ADDONS_NUEVO, Some(ADDONS_TXT)),
        escritura("config-cache.wtf", true, CONFIG_CUENTA, None),
    ]
}

fn importar(e: &Escenario) -> String {
    e.operaciones
        .importar(
            &e.personaje,
            paquete(),
            tres(),
            Vec::new(),
            &sin_carpetas,
            || false,
        )
        .unwrap()
}

fn cuenta(e: &Escenario) -> PathBuf {
    e.personaje.carpeta_cuenta()
}

fn sin_operaciones(datos: &Path) -> bool {
    !datos.exists() || subcarpetas(datos).unwrap().is_empty()
}

fn texto(ruta: &Path) -> String {
    read_to_string(ruta).unwrap()
}

#[test]
fn escribe_exactamente_los_bytes_del_plan() {
    let e = escenario();
    let id = importar(&e);

    let carpeta = e.personaje.carpeta();
    assert_eq!(texto(&carpeta.join("macros-cache.txt")), MACROS);
    assert_eq!(texto(&carpeta.join("AddOns.txt")), ADDONS_NUEVO);
    assert_eq!(texto(&cuenta(&e).join("config-cache.wtf")), CONFIG_CUENTA);
    assert_eq!(
        texto(&carpeta.join("SavedVariables/Prat-3.0.lua")),
        PRAT_LUA,
        "lo que no está en el plan no se toca"
    );
    let diario = diario_de(&e.datos, &id);
    assert_eq!(diario.estado, Estado::Completa);
    assert_eq!(diario.paquete, Some(paquete()));
    for cambio in &diario.cambios {
        let escrito = read(en_disco(&diario, cambio).unwrap()).unwrap();
        assert_eq!(Some(sha256(&escrito)), cambio.despues.sha256);
        assert_eq!(cambio.entrada.as_deref(), Some("e1"));
    }
    let copia = read(e.datos.join(&id).join("copias/0")).unwrap();
    assert_eq!(
        copia,
        ADDONS_TXT.as_bytes(),
        "solo el que existía tiene copia"
    );
    assert_eq!(diario.cambios[2].ruta, "WTF/Account/1#1/config-cache.wtf");
    assert!(diario.cambios[2].de_cuenta);
}

#[test]
fn el_diario_tiene_la_forma_del_schema() {
    let e = escenario();
    let id = importar(&e);
    let json: serde_json::Value =
        serde_json::from_slice(&read(e.datos.join(&id).join(DIARIO)).unwrap()).unwrap();

    assert_eq!(json["tipo"], "importacion");
    assert_eq!(json["paquete"]["nombre"], "Goldberg");
    assert!(json.get("deshace").is_none());
    assert_eq!(json["cambios"][0]["accion"], "crear");
    assert_eq!(json["cambios"][0]["antes"]["existia"], false);
    assert_eq!(json["cambios"][0]["entrada"], "e1");
    assert_eq!(json["cambios"][1]["accion"], "fusionar");
    assert_eq!(json["cambios"][1]["antes"]["existia"], true);
    assert_eq!(json["cambios"][2]["deCuenta"], true);
}

#[test]
fn si_el_disco_no_es_el_del_plan_no_crea_nada() {
    let e = escenario();
    write(e.personaje.carpeta().join("AddOns.txt"), "cambiado").unwrap();

    let resultado = e.operaciones.importar(
        &e.personaje,
        paquete(),
        tres(),
        Vec::new(),
        &sin_carpetas,
        || false,
    );

    assert_eq!(resultado, Err(ErrorNucleo::PlanCaducado {}));
    assert!(sin_operaciones(&e.datos));
    assert!(!e.personaje.carpeta().join("macros-cache.txt").exists());
}

#[test]
fn un_fichero_que_aparecio_desde_el_plan_no_se_pisa() {
    let e = escenario();
    let macros = e.personaje.carpeta().join("macros-cache.txt");
    write(&macros, "del jugador").unwrap();

    let resultado = e.operaciones.importar(
        &e.personaje,
        paquete(),
        tres(),
        Vec::new(),
        &sin_carpetas,
        || false,
    );

    assert_eq!(resultado, Err(ErrorNucleo::PlanCaducado {}));
    assert_eq!(texto(&macros), "del jugador");
}

#[test]
fn una_ruta_fuera_de_su_carpeta_se_rechaza() {
    let e = escenario();
    let vecino = e
        .personaje
        .raiz_version
        .join("WTF/Account/1#1/93/Goldberg-Joestar/AddOns.txt");
    for fichero in ["../Goldberg-Joestar/AddOns.txt", "C:/x.txt", "CON", "a./b"] {
        let resultado = e.operaciones.importar(
            &e.personaje,
            paquete(),
            vec![escritura(fichero, false, "y", None)],
            Vec::new(),
            &sin_carpetas,
            || false,
        );
        assert!(
            matches!(resultado, Err(ErrorNucleo::EscrituraFallida { .. })),
            "{fichero}: {resultado:?}"
        );
    }
    assert_eq!(texto(&vecino), "x");
    assert!(sin_operaciones(&e.datos));
}

#[test]
fn una_union_en_la_cuenta_se_rechaza_sin_tocar_nada() {
    let e = escenario();
    let fuera = e._tmp.path().join("fuera");
    create_dir_all(&fuera).unwrap();
    let union = cuenta(&e).join("SavedVariables");
    let creada = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&union)
        .arg(&fuera)
        .output()
        .unwrap();
    assert!(creada.status.success(), "{creada:?}");

    let resultado = e.operaciones.importar(
        &e.personaje,
        paquete(),
        vec![escritura("SavedVariables/X.lua", true, "x", None)],
        Vec::new(),
        &sin_carpetas,
        || false,
    );

    assert!(
        matches!(resultado, Err(ErrorNucleo::EnlaceNoAdmitido { .. })),
        "{resultado:?}"
    );
    assert!(!fuera.join("X.lua").exists());
    assert!(sin_operaciones(&e.datos));
    std::fs::remove_dir(&union).unwrap();
}

#[test]
fn con_el_juego_abierto_no_hace_nada() {
    let e = escenario();
    assert_eq!(
        e.operaciones.importar(
            &e.personaje,
            paquete(),
            tres(),
            Vec::new(),
            &sin_carpetas,
            || true
        ),
        Err(ErrorNucleo::JuegoAbierto {})
    );
    assert!(sin_operaciones(&e.datos));
}

/// El tercer fichero está abierto por otro programa sin dejar escribir: los dos primeros vuelven a como estaban.
#[test]
#[cfg(windows)]
fn si_falla_a_mitad_restaura_lo_escrito() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;

    let e = escenario();
    let config = cuenta(&e).join("config-cache.wtf");
    write(&config, CONFIG_VIEJA).unwrap();
    let mut escrituras = tres();
    escrituras[2] = escritura("config-cache.wtf", true, CONFIG_CUENTA, Some(CONFIG_VIEJA));
    let abierto = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&config)
        .unwrap();

    let resultado = e.operaciones.importar(
        &e.personaje,
        paquete(),
        escrituras,
        Vec::new(),
        &sin_carpetas,
        || false,
    );
    drop(abierto);

    let Err(ErrorNucleo::OperacionFallida { id, restaurada, .. }) = resultado else {
        panic!("{resultado:?}");
    };
    assert!(restaurada);
    let carpeta = e.personaje.carpeta();
    assert!(!carpeta.join("macros-cache.txt").exists());
    assert_eq!(texto(&carpeta.join("AddOns.txt")), ADDONS_TXT);
    assert_eq!(texto(&config), CONFIG_VIEJA);
    assert_eq!(diario_de(&e.datos, &id).estado, Estado::FallidaRestaurada);
}

/// Una importación cortada en `escribiendo`, con solo el primer fichero escrito.
fn a_medias(e: &Escenario) -> String {
    let id = importar(e);
    let mut diario = diario_de(&e.datos, &id);
    diario.estado = Estado::Escribiendo;
    diario.terminada = None;
    guardar(&e.datos.join(&id), &diario).unwrap();
    write(e.personaje.carpeta().join("AddOns.txt"), ADDONS_TXT).unwrap();
    std::fs::remove_file(cuenta(e).join("config-cache.wtf")).unwrap();
    id
}

#[test]
fn al_arrancar_una_importacion_a_medias_no_se_restaura_sola() {
    let e = escenario();
    let id = a_medias(&e);

    let recuperacion = e.operaciones.recuperar(|| false);

    assert_eq!(
        recuperacion,
        Recuperacion {
            a_medias: 1,
            ..Recuperacion::default()
        }
    );
    assert_eq!(diario_de(&e.datos, &id).estado, Estado::Escribiendo);
    assert!(e.personaje.carpeta().join("macros-cache.txt").is_file());
}

#[test]
fn una_importacion_a_medias_bloquea_las_demas_operaciones() {
    let e = escenario();
    let id = a_medias(&e);
    let bloqueo = Err(ErrorNucleo::OperacionAMedias { id });

    assert_eq!(
        e.operaciones.eliminar_personaje(&e.personaje, || false),
        bloqueo
    );
    assert_eq!(
        e.operaciones.importar(
            &e.personaje,
            paquete(),
            tres(),
            Vec::new(),
            &sin_carpetas,
            || false
        ),
        bloqueo
    );
    assert!(e.personaje.carpeta().join("AddOns.txt").is_file());
}

#[test]
fn restaurar_a_peticion_la_deja_como_antes() {
    let e = escenario();
    let id = a_medias(&e);

    e.operaciones.restaurar_operacion(&id, || false).unwrap();

    assert!(!e.personaje.carpeta().join("macros-cache.txt").exists());
    assert_eq!(texto(&e.personaje.carpeta().join("AddOns.txt")), ADDONS_TXT);
    assert_eq!(diario_de(&e.datos, &id).estado, Estado::FallidaRestaurada);
    assert!(
        e.operaciones
            .eliminar_personaje(&e.personaje, || false)
            .is_ok()
    );
}

#[test]
fn restaurar_a_peticion_no_sigue_una_union_puesta_despues() {
    let e = escenario();
    let id = a_medias(&e);
    let fuera = e._tmp.path().join("fuera");
    create_dir_all(&fuera).unwrap();
    let carpeta = e.personaje.carpeta();
    let movida = e._tmp.path().join("movida");
    std::fs::rename(&carpeta, &movida).unwrap();
    let creada = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&carpeta)
        .arg(&fuera)
        .output()
        .unwrap();
    assert!(creada.status.success(), "{creada:?}");

    let resultado = e.operaciones.restaurar_operacion(&id, || false);

    assert!(
        matches!(resultado, Err(ErrorNucleo::EnlaceNoAdmitido { .. })),
        "{resultado:?}"
    );
    assert_eq!(diario_de(&e.datos, &id).estado, Estado::Escribiendo);
    std::fs::remove_dir(&carpeta).unwrap();
}

/// Todo se escribe, pero el diario no se puede cerrar como `completa`: lo escrito se devuelve (rollback).
#[test]
fn si_no_se_puede_cerrar_el_diario_se_devuelve_lo_escrito() {
    let e = escenario();
    let (diario, contenidos) = preparar(&e.personaje, paquete(), tres()).unwrap();
    let (id, dir) = e.operaciones.nueva_carpeta().unwrap();
    let diario = Diario { id, ..diario };
    guardar(&dir, &diario).unwrap();

    let resultado = super::super::ejecutar(&dir, diario, &|| false, |d| {
        escribir(d, &contenidos)?;
        // El temporal del diario es una carpeta: guardarlo falla a partir de aquí.
        create_dir_all(dir.join(format!("{DIARIO}.tmp"))).unwrap();
        Ok(())
    });

    let Err(ErrorNucleo::OperacionFallida {
        restaurada,
        detalle,
        ..
    }) = resultado
    else {
        panic!("{resultado:?}");
    };
    assert!(restaurada, "{detalle}");
    assert!(
        detalle.contains("el diario no se pudo guardar"),
        "{detalle}"
    );
    let carpeta = e.personaje.carpeta();
    assert!(!carpeta.join("macros-cache.txt").exists());
    assert_eq!(texto(&carpeta.join("AddOns.txt")), ADDONS_TXT);
    assert!(!cuenta(&e).join("config-cache.wtf").exists());
}

/// Una operación que no se pudo restaurar del todo también bloquea las demás, y se puede reintentar.
#[test]
fn una_sin_restaurar_bloquea_y_se_reintenta() {
    let e = escenario();
    let id = importar(&e);
    let mut diario = diario_de(&e.datos, &id);
    diario.estado = Estado::FallidaSinRestaurar;
    guardar(&e.datos.join(&id), &diario).unwrap();

    assert_eq!(
        e.operaciones.eliminar_personaje(&e.personaje, || false),
        Err(ErrorNucleo::OperacionAMedias { id: id.clone() })
    );
    e.operaciones.restaurar_operacion(&id, || false).unwrap();

    assert_eq!(diario_de(&e.datos, &id).estado, Estado::FallidaRestaurada);
    assert_eq!(texto(&e.personaje.carpeta().join("AddOns.txt")), ADDONS_TXT);
    assert!(
        e.operaciones
            .eliminar_personaje(&e.personaje, || false)
            .is_ok()
    );
}

#[test]
fn solo_se_restaura_a_peticion_una_importacion_a_medias() {
    let e = escenario();
    let completa = importar(&e);
    assert_eq!(
        e.operaciones.restaurar_operacion(&completa, || false),
        Err(ErrorNucleo::NoRestaurable {})
    );
    let e = escenario();
    let id = a_medias(&e);
    assert_eq!(
        e.operaciones.restaurar_operacion(&id, || true),
        Err(ErrorNucleo::JuegoAbierto {})
    );
}

/// Todo lo que hay bajo la cuenta, con su contenido.
fn arbol(e: &Escenario) -> Vec<(PathBuf, Vec<u8>)> {
    ficheros_de(&cuenta(e))
        .unwrap()
        .into_iter()
        .map(|f| {
            let bytes = read(&f).unwrap();
            (f, bytes)
        })
        .collect()
}

#[test]
fn deshacer_una_importacion_deja_la_cuenta_byte_a_byte_como_antes() {
    let e = escenario();
    let antes = arbol(&e);
    let id = importar(&e);

    let plan = e.operaciones.plan_deshacer(&id).unwrap();
    assert_eq!((plan.vuelven, plan.sobran), (1, 2));
    assert_eq!(plan.de_cuenta, ["WTF/Account/1#1/config-cache.wtf"]);
    e.operaciones.deshacer(&id, &plan.huella, || false).unwrap();

    assert_eq!(arbol(&e), antes);
    assert_eq!(diario_de(&e.datos, &id).estado, Estado::Deshecha);
}

#[test]
fn deshacer_una_importacion_no_toca_lo_que_no_importo() {
    let e = escenario();
    let id = importar(&e);
    let nuevo = e.personaje.carpeta().join("SavedVariables/Nuevo.lua");
    write(&nuevo, "del juego").unwrap();
    std::fs::remove_file(e.personaje.carpeta().join("macros-cache.txt")).unwrap();

    let plan = e.operaciones.plan_deshacer(&id).unwrap();
    assert_eq!(
        (plan.vuelven, plan.sobran),
        (1, 1),
        "el creado y ya borrado no cuenta"
    );
    e.operaciones.deshacer(&id, &plan.huella, || false).unwrap();

    assert_eq!(texto(&nuevo), "del juego");
    assert_eq!(texto(&e.personaje.carpeta().join("AddOns.txt")), ADDONS_TXT);
    assert!(!cuenta(&e).join("config-cache.wtf").exists());
}

#[test]
fn una_importacion_posterior_en_otro_personaje_de_la_cuenta_bloquea_deshacer() {
    let e = escenario();
    let primera = importar(&e);
    let mut vecino = e.personaje.clone();
    vecino.personaje = "Goldberg-Joestar".into();
    let segunda = e
        .operaciones
        .importar(
            &vecino,
            paquete(),
            vec![escritura(
                "config-cache.wtf",
                true,
                "SET b \"1\"\r\n",
                Some(CONFIG_CUENTA),
            )],
            Vec::new(),
            &sin_carpetas,
            || false,
        )
        .unwrap();

    assert_eq!(
        e.operaciones.plan_deshacer(&primera),
        Err(ErrorNucleo::OperacionPosterior { id: segunda })
    );
}

#[test]
fn el_historial_dice_el_perfil_y_si_esta_a_medias() {
    let e = escenario();
    importar(&e);
    let historial = serde_json::to_value(e.operaciones.listar()).unwrap();
    let completa = &historial["operaciones"][0];
    assert_eq!(completa["paquete"]["nombre"], "Goldberg");
    assert_eq!(completa["deshacible"], true);
    assert_eq!(completa["aMedias"], false);

    let e = escenario();
    a_medias(&e);
    let historial = serde_json::to_value(e.operaciones.listar()).unwrap();
    let cortada = &historial["operaciones"][0];
    assert_eq!(cortada["aMedias"], true);
    assert_eq!(cortada["deshacible"], false);
    assert_eq!(cortada["sinDevolver"].as_array().unwrap().len(), 1);
}

/// Una importación sin carpetas de addons no carga ninguna.
fn sin_carpetas(_: &str) -> Result<Vec<crate::addon_zip::FicheroAddon>, ErrorNucleo> {
    unreachable!("sin carpetas no se carga nada")
}
