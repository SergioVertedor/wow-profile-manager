//! Contrato entre la interfaz (`src/nucleo.ts`) y el núcleo: los comandos se llaman por su nombre, como hace
//! `invoke`. Usa el runtime simulado de Tauri, sin abrir ventanas.

use std::io::Write;
use std::path::Path;
use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use wow_profile_manager_lib::{
    Almacen, CARPETA_OPERACIONES, NOMBRE_APP, Operaciones, PaquetesAbiertos, Registro, con_comandos,
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
    invocar_con_paquetes(datos, PaquetesAbiertos::default(), comando, argumentos)
}

/// Como `invocar_en`, con un paquete ya abierto (el diálogo de abrir no tiene quien lo cierre en el runtime simulado).
fn invocar_con_paquetes(
    datos: &Path,
    paquetes: PaquetesAbiertos,
    comando: &str,
    argumentos: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let app = con_comandos(
        mock_builder()
            .manage(Almacen::en(datos.to_path_buf()))
            .manage(Operaciones::en(datos.join(CARPETA_OPERACIONES)))
            .manage(paquetes)
            // La app instala uno solo para toda su vida; aquí, uno por test, que vive lo que dura el proceso.
            .manage::<&'static Registro>(Box::leak(Box::new(Registro::en(
                datos.join("registros"),
                false,
            )))),
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
fn el_interruptor_del_registro_se_guarda_en_las_preferencias() {
    let tmp = tempfile::tempdir().unwrap();
    let estado = invocar_en(tmp.path(), "estado_diagnostico", serde_json::json!({})).unwrap();
    assert_eq!(estado["guarda"], false);
    assert!(estado["lineas"].is_array());

    invocar_en(
        tmp.path(),
        "cambiar_registro",
        serde_json::json!({ "guardar": true }),
    )
    .unwrap();
    let preferencias: serde_json::Value =
        serde_json::from_slice(&std::fs::read(tmp.path().join("preferencias.json")).unwrap())
            .unwrap();
    assert_eq!(preferencias["registro"], true);
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
            "canceladas": 0, "restauradas": 0, "sinRestaurar": 0, "pendientes": 0, "aMedias": 0,
            "ilegibles": 0
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
        ("restaurar_operacion", serde_json::json!({ "id": ".." })),
    ] {
        let error = invocar_en(tmp.path(), comando, argumentos).unwrap_err();
        // Deshacer mira antes si el juego está abierto: si quien corre los tests está jugando, esa es la respuesta.
        assert!(
            error["codigo"] == "operacionNoEncontrada" || error["codigo"] == "juegoAbierto",
            "{comando}: {error}"
        );
    }
}

#[test]
fn propuesta_exportacion_lista_las_piezas_del_personaje() {
    let tmp = tempfile::tempdir().unwrap();
    let (raiz, datos) = (tmp.path().join("wow"), tmp.path().join("datos"));
    std::fs::create_dir_all(&raiz).unwrap();
    let pedido = instalacion_con_personaje(&raiz, &datos);

    let propuesta = invocar_en(&datos, "propuesta_exportacion", pedido).unwrap();

    assert_eq!(propuesta["piezas"][0]["id"], "addons-activos");
    assert_eq!(propuesta["piezas"][0]["categoria"], "addons-activos");
    assert_eq!(propuesta["piezas"][0]["ambito"], "personaje");
    // Prat-3.0 no está instalado en el árbol de prueba: su línea se quita, con su motivo.
    assert_eq!(
        propuesta["piezas"][0]["quitadas"],
        serde_json::json!([{ "motivo": "addonNoInstalado", "claves": ["Prat-3.0"] }])
    );
    assert_eq!(propuesta["apartadas"], serde_json::json!([]));
    assert!(propuesta["juegoAbierto"].is_boolean());
}

/// Solo los casos que fallan antes de abrir el diálogo de guardar: el runtime simulado no tiene quien lo cierre.
#[test]
fn exportar_personaje_valida_antes_de_abrir_el_dialogo() {
    let tmp = tempfile::tempdir().unwrap();
    let (raiz, datos) = (tmp.path().join("wow"), tmp.path().join("datos"));
    std::fs::create_dir_all(&raiz).unwrap();
    let mut pedido = instalacion_con_personaje(&raiz, &datos);
    pedido["seleccion"] = serde_json::json!(["addons-activos"]);
    pedido["nombre"] = serde_json::json!("Druida");
    pedido["descripcion"] = serde_json::json!("");

    let error = invocar_en(&datos, "exportar_personaje", pedido.clone()).unwrap_err();
    assert_eq!(error["codigo"], "exportacionSinVersion");

    std::fs::write(
        raiz.join(".build.info"),
        "Version!STRING:0|Product!STRING:0\n1.60.1.70938|wowf\n",
    )
    .unwrap();
    std::fs::write(
        raiz.join("_forever_/WTF/Config.wtf"),
        "SET engineSurveyPatch \"16001\"\r\n",
    )
    .unwrap();
    pedido["seleccion"] = serde_json::json!([]);
    let error = invocar_en(&datos, "exportar_personaje", pedido).unwrap_err();
    assert_eq!(error["codigo"], "seleccionVacia");
}

/// Un `.wowprofile` de Forever con una sola entrada: `AddOns.txt` con Prat-3.0 desactivado.
fn paquete_de_prueba(ruta: &Path) {
    use sha2::{Digest, Sha256};
    let addons = b"Prat-3.0: disabled\r\n";
    let hash: String = Sha256::digest(addons)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let manifiesto = serde_json::json!({
        "formato": { "nombre": "wowprofile", "version": "1.0" },
        "perfil": { "nombre": "Druida", "creado": "2026-11-18T10:00:00Z" },
        "origen": { "producto": "wowf", "build": "1.60.1.70938", "interfaz": 16001 },
        "addons": [],
        "entradas": [{
            "id": "addons-activos", "categoria": "addons-activos", "tipo": "fichero-filtrado",
            "ambito": "personaje", "destino": "personaje/AddOns.txt",
            "sha256": hash, "bytes": addons.len(), "claves_quitadas": []
        }]
    });
    let mut zip = zip::ZipWriter::new(std::fs::File::create(ruta).unwrap());
    let opciones = zip::write::SimpleFileOptions::default();
    zip.start_file("manifest.json", opciones).unwrap();
    zip.write_all(manifiesto.to_string().as_bytes()).unwrap();
    zip.start_file("datos/addons-activos", opciones).unwrap();
    zip.write_all(addons).unwrap();
    zip.finish().unwrap();
}

/// La instalación de `instalacion_con_personaje` y un paquete abierto. Devuelve los argumentos de `plan_importacion`.
fn importacion_preparada(raiz: &Path, datos: &Path) -> (PaquetesAbiertos, serde_json::Value) {
    let personaje = instalacion_con_personaje(raiz, datos)["personaje"].clone();
    let ruta = raiz.join("perfil.wowprofile");
    paquete_de_prueba(&ruta);
    let paquetes = PaquetesAbiertos::default();
    let resumen = serde_json::to_value(paquetes.abrir(&ruta).unwrap()).unwrap();
    let argumentos = serde_json::json!({
        "sha256": resumen["sha256"],
        "personajes": [personaje],
        "pedido": { "seleccion": ["addons-activos"], "cuentas": [] }
    });
    (paquetes, argumentos)
}

#[test]
fn plan_importacion_ensena_que_cambia_sin_escribir() {
    let tmp = tempfile::tempdir().unwrap();
    let (raiz, datos) = (tmp.path().join("wow"), tmp.path().join("datos"));
    std::fs::create_dir_all(&raiz).unwrap();
    let (paquetes, argumentos) = importacion_preparada(&raiz, &datos);
    let addons_txt = raiz.join("_forever_/WTF/Account/1#1/93/Goldberg-Soulreaper/AddOns.txt");

    let vista = invocar_con_paquetes(&datos, paquetes, "plan_importacion", argumentos).unwrap();

    let fichero = &vista["plan"]["personajes"][0]["ficheros"][0];
    assert_eq!(fichero["entrada"], "addons-activos");
    assert_eq!(fichero["grupo"], "modificado");
    assert_eq!(fichero["accion"], "fusionar");
    assert_eq!(fichero["elementos"][0]["nombre"], "Prat-3.0");
    assert_eq!(fichero["elementos"][0]["despues"], "disabled");
    assert!(
        fichero.get("ruta").is_none() && fichero.get("contenido").is_none(),
        "{fichero}"
    );
    assert!(vista["juegoAbierto"].is_boolean());
    assert_eq!(std::fs::read(addons_txt).unwrap(), b"Prat-3.0: enabled\r\n");
}

#[test]
fn plan_importacion_rechaza_otro_paquete_y_destinos_no_validos() {
    let tmp = tempfile::tempdir().unwrap();
    let (raiz, datos) = (tmp.path().join("wow"), tmp.path().join("datos"));
    std::fs::create_dir_all(&raiz).unwrap();
    let (_, argumentos) = importacion_preparada(&raiz, &datos);
    let personaje = argumentos["personajes"][0].clone();
    let casos = [
        (
            "0".repeat(64),
            serde_json::json!([personaje]),
            "paqueteNoAbierto",
        ),
        (String::new(), serde_json::json!([]), "destinosNoValidos"),
        (
            String::new(),
            serde_json::json!([personaje, personaje]),
            "destinosNoValidos",
        ),
    ];
    for (sha, personajes, codigo) in casos {
        let (paquetes, mut argumentos) = importacion_preparada(&raiz, &datos);
        if !sha.is_empty() {
            argumentos["sha256"] = serde_json::json!(sha);
        }
        argumentos["personajes"] = personajes;
        let error =
            invocar_con_paquetes(&datos, paquetes, "plan_importacion", argumentos).unwrap_err();
        assert_eq!(error["codigo"], codigo);
    }
}

/// Aplicar con la huella de la vista previa escribe, sale en el historial con su perfil y se deshace desde allí. Mira los
/// procesos reales: con el juego abierto, el resultado del personaje lo dice y no se escribe nada.
#[test]
fn aplicar_importacion_escribe_y_se_deshace_desde_el_historial() {
    let tmp = tempfile::tempdir().unwrap();
    let (raiz, datos) = (tmp.path().join("wow"), tmp.path().join("datos"));
    std::fs::create_dir_all(&raiz).unwrap();
    let (paquetes, mut argumentos) = importacion_preparada(&raiz, &datos);
    let addons_txt = raiz.join("_forever_/WTF/Account/1#1/93/Goldberg-Soulreaper/AddOns.txt");
    let vista =
        invocar_con_paquetes(&datos, paquetes, "plan_importacion", argumentos.clone()).unwrap();
    // El runtime simulado no guarda estado entre llamadas: el mismo fichero, abierto otra vez.
    let abiertos = PaquetesAbiertos::default();
    abiertos.abrir(&raiz.join("perfil.wowprofile")).unwrap();
    argumentos["huella"] = vista["huella"].clone();

    let resultados =
        invocar_con_paquetes(&datos, abiertos, "aplicar_importacion", argumentos).unwrap();

    let resultado = &resultados[0];
    assert_eq!(resultado["personaje"], "Goldberg-Soulreaper");
    if resultado["resultado"] == "fallida" {
        assert_eq!(resultado["error"]["codigo"], "juegoAbierto", "{resultado}");
        assert_eq!(
            std::fs::read(&addons_txt).unwrap(),
            b"Prat-3.0: enabled\r\n"
        );
        return;
    }
    assert_eq!(resultado["resultado"], "completa", "{resultado}");
    assert_eq!(resultado["ficheros"], 1);
    assert_eq!(
        std::fs::read(&addons_txt).unwrap(),
        b"Prat-3.0: disabled\r\n"
    );
    let id = resultado["operacion"].clone();
    let historial = invocar_en(&datos, "listar_operaciones", serde_json::json!({})).unwrap();
    let operacion = &historial["operaciones"][0];
    assert_eq!(operacion["tipo"], "importacion");
    assert_eq!(operacion["paquete"]["nombre"], "Druida");
    assert_eq!(operacion["deshacible"], true);

    let plan = invocar_en(&datos, "plan_deshacer", serde_json::json!({ "id": id })).unwrap();
    assert_eq!(plan["vuelven"], 1);
    let argumentos = serde_json::json!({ "id": id, "huella": plan["huella"] });
    match invocar_en(&datos, "deshacer_operacion", argumentos) {
        Ok(_) => assert_eq!(
            std::fs::read(&addons_txt).unwrap(),
            b"Prat-3.0: enabled\r\n"
        ),
        Err(error) => assert_eq!(error["codigo"], "juegoAbierto"),
    }
}

#[test]
fn aplicar_importacion_con_otra_huella_no_escribe() {
    let tmp = tempfile::tempdir().unwrap();
    let (raiz, datos) = (tmp.path().join("wow"), tmp.path().join("datos"));
    std::fs::create_dir_all(&raiz).unwrap();
    let (paquetes, mut argumentos) = importacion_preparada(&raiz, &datos);
    argumentos["huella"] = serde_json::json!("0".repeat(64));

    let error =
        invocar_con_paquetes(&datos, paquetes, "aplicar_importacion", argumentos).unwrap_err();

    assert_eq!(error["codigo"], "planCaducado");
    assert!(!datos.join(CARPETA_OPERACIONES).exists());
}

#[test]
fn cerrar_paquete_responde_por_su_nombre() {
    assert_eq!(invocar("cerrar_paquete").unwrap(), serde_json::Value::Null);
}
