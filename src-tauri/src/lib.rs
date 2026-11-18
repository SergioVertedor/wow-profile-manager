//! Núcleo de WoW Profile Manager. Los módulos siguen `docs/arquitectura.md`; los comandos de Tauri son la única
//! puerta de entrada desde la interfaz y no contienen lógica: delegan en módulos que se prueban sin Tauri.

mod addons;
mod bpsv;
mod claves_personaje;
mod descubrimiento;
mod deteccion;
mod disco;
mod enlaces;
mod error;
mod idioma;
mod info;
mod preferencias;
mod toc;

pub use info::NOMBRE_APP;
pub use preferencias::Almacen;

/// Etiqueta de la ventana de `tauri.conf.json`; el selector de carpetas se abre como hijo suyo.
const VENTANA_PRINCIPAL: &str = "main";

use std::path::PathBuf;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

/// Devuelve el nombre y la versión de la aplicación.
#[tauri::command]
fn info_app() -> info::InfoApp {
    info::info_app()
}

/// Devuelve el idioma de la interfaz de Windows (`"es"` o `"en"`). Los mensajes del núcleo no se traducen aquí:
/// viajan como un código y sus datos, y la interfaz los traduce con `src/textos/`.
#[tauri::command]
fn idioma_interfaz() -> idioma::Idioma {
    idioma::idioma_sistema()
}

/// Busca las instalaciones de WoW del equipo (registro y carpetas a mano) con sus versiones, cuentas, personajes y
/// los nombres que les ha puesto el usuario. No escribe nada. Es `async` para que leer el disco no congele la ventana.
#[tauri::command]
async fn detectar_instalaciones(
    almacen: State<'_, Almacen>,
) -> Result<deteccion::Deteccion, error::ErrorNucleo> {
    deteccion::detectar_en_el_sistema(almacen.leer())
}

/// Abre el selector de carpetas de Windows y, si la elegida es una instalación nueva, la recuerda. El selector se
/// abre desde aquí: la interfaz no tiene permiso para abrir diálogos ni para elegir rutas. Es `async` para que el
/// selector, que espera al usuario, no bloquee el hilo de la ventana.
#[tauri::command]
async fn anadir_carpeta<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    almacen: State<'_, Almacen>,
) -> Result<deteccion::Alta, error::ErrorNucleo> {
    let mut selector = app.dialog().file();
    if let Some(ventana) = app.get_webview_window(VENTANA_PRINCIPAL) {
        selector = selector.set_parent(&ventana);
    }
    let Some(raiz) = selector
        .blocking_pick_folder()
        .and_then(|c| c.into_path().ok())
    else {
        return Ok(deteccion::Alta::Cancelada);
    };
    let registro = deteccion::rutas_del_registro();
    almacen.cambiar(|p| Ok(deteccion::dar_de_alta(&raiz, &registro, p)))
}

/// Olvida una carpeta añadida a mano, con su nombre y los de sus cuentas. No toca la carpeta.
#[tauri::command]
async fn quitar_carpeta(
    raiz: PathBuf,
    almacen: State<'_, Almacen>,
) -> Result<(), error::ErrorNucleo> {
    almacen.cambiar(|p| {
        p.quitar_carpeta(&raiz);
        Ok(())
    })
}

/// Pone, cambia o borra (texto vacío) el nombre propio de una instalación.
#[tauri::command]
async fn nombrar_instalacion(
    raiz: PathBuf,
    nombre: String,
    almacen: State<'_, Almacen>,
) -> Result<(), error::ErrorNucleo> {
    let nombre = preferencias::nombre_valido(&nombre)?;
    almacen.cambiar(|p| {
        p.nombrar_instalacion(&raiz, nombre);
        Ok(())
    })
}

/// Pone, cambia o borra (texto vacío) el nombre propio de una cuenta de una versión de una instalación.
#[tauri::command]
async fn nombrar_cuenta(
    raiz: PathBuf,
    version: String,
    cuenta: String,
    nombre: String,
    almacen: State<'_, Almacen>,
) -> Result<(), error::ErrorNucleo> {
    let nombre = preferencias::nombre_valido(&nombre)?;
    almacen.cambiar(|p| {
        p.nombrar_cuenta(&raiz, &version, &cuenta, nombre);
        Ok(())
    })
}

/// Abre en el navegador uno de los enlaces conocidos. La interfaz no puede pasar una URL: solo el nombre del enlace.
#[tauri::command]
fn abrir_enlace(enlace: enlaces::Enlace) -> Result<(), error::ErrorNucleo> {
    enlaces::abrir(enlace)
}

/// Registra los comandos que puede llamar la interfaz. Es genérico sobre el runtime para que los tests de
/// `tests/comandos.rs` usen el runtime simulado de Tauri y llamen a los comandos por su nombre, igual que la
/// interfaz. Los comandos de preferencias necesitan un `Almacen` gestionado: `run` lo pone en la carpeta de datos de
/// la app y los tests, en una carpeta temporal.
pub fn con_comandos<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        info_app,
        idioma_interfaz,
        detectar_instalaciones,
        anadir_carpeta,
        quitar_carpeta,
        nombrar_instalacion,
        nombrar_cuenta,
        abrir_enlace
    ])
}

/// Arranca la aplicación. Si Tauri no puede abrir la ventana, no hay nada que hacer salvo terminar con un error.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let resultado = con_comandos(tauri::Builder::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(Almacen::en(app.path().app_data_dir()?));
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(error) = resultado {
        eprintln!("{NOMBRE_APP}: no se pudo arrancar: {error}");
        std::process::exit(1);
    }
}
