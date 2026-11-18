//! Contrato entre la interfaz (`src/nucleo.ts`) y el núcleo: los comandos se llaman por su nombre, como hace
//! `invoke`. Usa el runtime simulado de Tauri, sin abrir ventanas.

use std::path::Path;
use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use wow_profile_manager_lib::{
    Almacen, CARPETA_OPERACIONES, NOMBRE_APP, Operaciones, con_comandos,
};

fn invocar(comando: &str) -> Result<serde_json::Value, serde_json::Value> {
    invocar_con(comando, serde_json::json!({}))
}

fn invocar_con(
    comando: &str,
    argumentos: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let tmp = tempfile::tempdir().unwrap();
    invocar_en(tmp.path(), comando, argumentos)
}

/// Invoca con las preferencias en `datos`: los tests nunca tocan la carpeta de datos real de la app.
fn invocar_en(
    datos: &Path,
    comando: &str,
    argumentos: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let app = con_comandos(
        mock_builder()
            .manage(Almacen::en(datos.to_path_buf()))
            .manage(Operaciones::en(datos.join(CARPETA_OPERACIONES))),
    )
    .build(mock_context(noop_assets()))
    .unwrap();
    let ventana = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let peticion = InvokeRequest {
        cmd: comando.into(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: "http://tauri.localhost".parse().unwrap(),
        body: tauri::ipc::InvokeBody::Json(argumentos),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    };
    get_ipc_response(&ventana, peticion).map(|cuerpo| cuerpo.deserialize().unwrap())
}

#[test]
fn info_app_responde_por_su_nombre_con_los_campos_de_la_interfaz() {
    let respuesta = invocar("info_app").unwrap();
    assert_eq!(respuesta["nombre"], NOMBRE_APP);
    assert_eq!(respuesta["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn idioma_interfaz_responde_es_o_en() {
    let respuesta = invocar("idioma_interfaz").unwrap();
    assert!(respuesta == "es" || respuesta == "en", "{respuesta}");
}

#[test]
fn abrir_enlace_rechaza_lo_que_no_es_un_enlace_conocido() {
    // Solo casos rechazados: uno válido abriría el navegador de quien corre los tests.
    // El rechazo tiene que venir del enum `Enlace` (variante desconocida), no de que el comando no exista.
    for enlace in ["https://example.com", "otro"] {
        let error =
            invocar_con("abrir_enlace", serde_json::json!({ "enlace": enlace })).unwrap_err();
        assert!(error.to_string().contains("unknown variant"), "{error}");
    }
}

#[test]
fn un_comando_que_no_existe_da_error() {
    assert!(invocar("no_existe").is_err());
}

#[test]
fn los_nombres_se_guardan_en_la_carpeta_de_datos_y_se_quitan_con_la_carpeta() {
    let tmp = tempfile::tempdir().unwrap();
    let raiz = r"E:\Juegos\WoW";
    invocar_en(
        tmp.path(),
        "nombrar_instalacion",
        serde_json::json!({ "raiz": raiz, "nombre": "  Portátil " }),
    )
    .unwrap();
    invocar_en(
        tmp.path(),
        "nombrar_cuenta",
        serde_json::json!({ "raiz": raiz, "version": "_retail_", "cuenta": "1#1", "nombre": "Alts" }),
    )
    .unwrap();

    let leer = || -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(tmp.path().join("preferencias.json")).unwrap())
            .unwrap()
    };
    let guardado = leer();
    assert_eq!(guardado["nombresInstalacion"][r"e:\juegos\wow"], "Portátil");
    assert_eq!(
        guardado["nombresCuenta"][r"e:\juegos\wow|_retail_|1#1"],
        "Alts"
    );

    invocar_en(
        tmp.path(),
        "quitar_carpeta",
        serde_json::json!({ "raiz": raiz }),
    )
    .unwrap();

    let guardado = leer();
    assert_eq!(guardado["nombresInstalacion"], serde_json::json!({}));
    assert_eq!(guardado["nombresCuenta"], serde_json::json!({}));
}

#[test]
fn un_nombre_no_valido_se_rechaza_con_su_codigo_y_no_escribe() {
    let tmp = tempfile::tempdir().unwrap();
    let error = invocar_en(
        tmp.path(),
        "nombrar_instalacion",
        serde_json::json!({ "raiz": "C:/WoW", "nombre": "a".repeat(41) }),
    )
    .unwrap_err();
    assert_eq!(error["codigo"], "nombreNoValido");
    assert_eq!(error["datos"]["maximo"], 40);
    assert!(!tmp.path().join("preferencias.json").exists());
}

/// Una instalación añadida a mano en `raiz` con un personaje de Forever de un fichero, y las preferencias en `datos`
/// que la conocen. Devuelve los argumentos con los que la interfaz pide ese personaje.
fn instalacion_con_personaje(raiz: &Path, datos: &Path) -> serde_json::Value {
    std::fs::write(raiz.join(".build.info"), "Product!STRING:0\nwowf\n").unwrap();
    let version = raiz.join("_forever_");
    let personaje = version.join("WTF/Account/1#1/93/Goldberg-Soulreaper");
    std::fs::create_dir_all(&personaje).unwrap();
    std::fs::write(
        version.join(".flavor.info"),
        "Product Flavor!STRING:0\nwowf\n",
    )
    .unwrap();
    std::fs::write(personaje.join("AddOns.txt"), "Prat-3.0: enabled\r\n").unwrap();
    std::fs::create_dir_all(datos).unwrap();
    std::fs::write(
        datos.join("preferencias.json"),
        serde_json::json!({ "version": 1, "carpetasManuales": [raiz] }).to_string(),
    )
    .unwrap();
    serde_json::json!({ "personaje": {
        "raiz": raiz, "version": "_forever_", "cuenta": "1#1", "reino": "93", "personaje": "Goldberg-Soulreaper"
    } })
}

#[test]
fn resumen_eliminacion_cuenta_los_ficheros_del_personaje() {
    let tmp = tempfile::tempdir().unwrap();
    let (raiz, datos) = (tmp.path().join("wow"), tmp.path().join("datos"));
    std::fs::create_dir_all(&raiz).unwrap();
    let pedido = instalacion_con_personaje(&raiz, &datos);

    let resumen = invocar_en(&datos, "resumen_eliminacion", pedido).unwrap();

    assert_eq!(resumen["ficheros"], 1);
    assert_eq!(resumen["bytes"], 19);
    assert!(
        resumen["ruta"]
            .as_str()
            .unwrap()
            .ends_with("Goldberg-Soulreaper")
    );
}

#[test]
fn eliminar_personaje_saca_la_carpeta_o_dice_que_el_juego_esta_abierto() {
    let tmp = tempfile::tempdir().unwrap();
    let (raiz, datos) = (tmp.path().join("wow"), tmp.path().join("datos"));
    std::fs::create_dir_all(&raiz).unwrap();
    let pedido = instalacion_con_personaje(&raiz, &datos);
    let carpeta = raiz.join("_forever_/WTF/Account/1#1/93/Goldberg-Soulreaper");

    // Mira los procesos reales: si quien corre los tests está jugando, la respuesta correcta es «juego abierto».
    match invocar_en(&datos, "eliminar_personaje", pedido) {
        Ok(id) => {
            assert!(!carpeta.exists());
            let diario = datos.join(CARPETA_OPERACIONES).join(id.as_str().unwrap());
            assert!(diario.join("operacion.json").is_file());
        }
        Err(error) => {
            assert_eq!(error["codigo"], "juegoAbierto");
            assert!(carpeta.join("AddOns.txt").is_file());
        }
    }
}

#[test]
fn eliminar_un_personaje_que_no_se_lista_se_rechaza_sin_escribir() {
    let tmp = tempfile::tempdir().unwrap();
    let (raiz, datos) = (tmp.path().join("wow"), tmp.path().join("datos"));
    std::fs::create_dir_all(&raiz).unwrap();
    let mut pedido = instalacion_con_personaje(&raiz, &datos);
    pedido["personaje"]["personaje"] = "../../../../..".into();

    let error = invocar_en(&datos, "eliminar_personaje", pedido).unwrap_err();

    assert_eq!(error["codigo"], "personajeNoEncontrado");
    assert!(!datos.join(CARPETA_OPERACIONES).exists());
}

#[test]
fn recuperar_operaciones_sin_operaciones_no_hace_nada() {
    let respuesta = invocar("recuperar_operaciones").unwrap();
    assert_eq!(
        respuesta,
        serde_json::json!({
            "canceladas": 0, "restauradas": 0, "sinRestaurar": 0, "pendientes": 0, "ilegibles": 0
        })
    );
}

/// El historial completo por IPC, con los nombres que usa la interfaz: eliminar, listar, planear, deshacer y borrar.
#[test]
fn el_historial_lista_deshace_y_borra_por_su_nombre() {
    let tmp = tempfile::tempdir().unwrap();
    let (raiz, datos) = (tmp.path().join("wow"), tmp.path().join("datos"));
    std::fs::create_dir_all(&raiz).unwrap();
    let pedido = instalacion_con_personaje(&raiz, &datos);
    let carpeta = raiz.join("_forever_/WTF/Account/1#1/93/Goldberg-Soulreaper");
    // Mira los procesos reales: con el juego abierto no hay nada que deshacer (lo cubre el test de eliminar).
    let id = match invocar_en(&datos, "eliminar_personaje", pedido) {
        Ok(id) => id,
        Err(error) => {
            assert_eq!(error["codigo"], "juegoAbierto", "{error}");
            return;
        }
    };

    let historial = invocar_en(&datos, "listar_operaciones", serde_json::json!({})).unwrap();
    let operacion = &historial["operaciones"][0];
    assert_eq!(operacion["id"], id);
    assert_eq!(operacion["tipo"], "eliminacion");
    assert_eq!(operacion["estado"], "completa");
    assert_eq!(operacion["deshacible"], true);
    assert_eq!(
        operacion["destino"]["carpetaPersonaje"],
        "Goldberg-Soulreaper"
    );
    assert_eq!(operacion["sinDevolver"], serde_json::json!([]));
    assert!(historial["bytes"].as_u64().unwrap() > 0);

    let plan = invocar_en(&datos, "plan_deshacer", serde_json::json!({ "id": id })).unwrap();
    assert_eq!(plan["vuelven"], 1);
    assert_eq!(plan["sobran"], 0);
    assert_eq!(plan["cambian"], serde_json::json!([]));
    let argumentos = serde_json::json!({ "id": id, "huella": plan["huella"] });
    match invocar_en(&datos, "deshacer_operacion", argumentos) {
        Ok(_) => assert!(carpeta.join("AddOns.txt").is_file()),
        Err(error) => assert_eq!(error["codigo"], "juegoAbierto"),
    }

    let argumentos = serde_json::json!({ "id": id, "restauradaAMano": false });
    invocar_en(&datos, "borrar_operacion", argumentos).unwrap();
    assert!(
        !datos
            .join(CARPETA_OPERACIONES)
            .join(id.as_str().unwrap())
            .exists()
    );
}

/// La interfaz solo manda ids: lo que no es una carpeta de la lista no se abre, no se planea y no se borra.
#[test]
fn un_id_de_operacion_desconocido_se_rechaza() {
    let tmp = tempfile::tempdir().unwrap();
    for (comando, argumentos) in [
        ("plan_deshacer", serde_json::json!({ "id": ".." })),
        (
            "deshacer_operacion",
            serde_json::json!({ "id": "..", "huella": "" }),
        ),
        (
            "borrar_operacion",
            serde_json::json!({ "id": "..", "restauradaAMano": true }),
        ),
        (
            "abrir_carpeta_operacion",
            serde_json::json!({ "id": "C:/Windows" }),
        ),
    ] {
        let error = invocar_en(tmp.path(), comando, argumentos).unwrap_err();
        // Deshacer mira antes si el juego está abierto: si quien corre los tests está jugando, esa es la respuesta.
        assert!(
            error["codigo"] == "operacionNoEncontrada" || error["codigo"] == "juegoAbierto",
            "{comando}: {error}"
        );
    }
}
