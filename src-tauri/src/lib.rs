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
mod toc;

pub use info::NOMBRE_APP;

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

/// Busca las instalaciones de WoW del equipo con sus versiones, cuentas y personajes. Solo lee. Es `async` para que
/// leer el disco no congele la ventana.
#[tauri::command]
async fn detectar_instalaciones() -> Result<Vec<deteccion::Instalacion>, error::ErrorNucleo> {
    deteccion::detectar_en_el_sistema()
}

/// Abre en el navegador uno de los enlaces conocidos. La interfaz no puede pasar una URL: solo el nombre del enlace.
#[tauri::command]
fn abrir_enlace(enlace: enlaces::Enlace) -> Result<(), error::ErrorNucleo> {
    enlaces::abrir(enlace)
}

/// Registra los comandos que puede llamar la interfaz. Es genérico sobre el runtime para que los tests de
/// `tests/comandos.rs` usen el runtime simulado de Tauri y llamen a los comandos por su nombre, igual que la
/// interfaz.
pub fn con_comandos<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        info_app,
        idioma_interfaz,
        detectar_instalaciones,
        abrir_enlace
    ])
}

/// Arranca la aplicación. Si Tauri no puede abrir la ventana, no hay nada que hacer salvo terminar con un error.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let resultado = con_comandos(tauri::Builder::default()).run(tauri::generate_context!());
    if let Err(error) = resultado {
        eprintln!("{NOMBRE_APP}: no se pudo arrancar: {error}");
        std::process::exit(1);
    }
}
