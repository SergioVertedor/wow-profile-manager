//! Contrato entre la interfaz (`src/nucleo.ts`) y el núcleo: los comandos se llaman por su nombre, como hace
//! `invoke`. Usa el runtime simulado de Tauri, sin abrir ventanas.

use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use wow_profile_manager_lib::{NOMBRE_APP, con_comandos};

fn invocar(comando: &str) -> Result<serde_json::Value, serde_json::Value> {
    let app = con_comandos(mock_builder())
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
        body: tauri::ipc::InvokeBody::default(),
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
fn un_comando_que_no_existe_da_error() {
    assert!(invocar("no_existe").is_err());
}
