//! Contrato entre la interfaz (`src/nucleo.ts`) y el núcleo: los comandos se llaman por su nombre, como hace
//! `invoke`. Usa el runtime simulado de Tauri, sin abrir ventanas.

use std::path::Path;
use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use wow_profile_manager_lib::{Almacen, NOMBRE_APP, con_comandos};

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
    let app = con_comandos(mock_builder().manage(Almacen::en(datos.to_path_buf())))
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
