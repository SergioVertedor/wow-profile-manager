//! Núcleo de WoW Profile Manager. Los módulos siguen `docs/arquitectura.md`; los comandos de Tauri son la única
//! puerta de entrada desde la interfaz y no contienen lógica: delegan en módulos que se prueban sin Tauri.

mod addons;
mod agente_battlenet;
mod bpsv;
mod claves_personaje;
mod descubrimiento;
mod deteccion;
mod disco;
mod enlaces;
mod error;
mod escritura;
mod exportacion;
mod idioma;
mod importacion;
mod info;
mod operaciones;
mod paquetes;
mod piezas;
mod preferencias;
mod sistema;
mod toc;

pub use importacion::PaquetesAbiertos;
pub use info::NOMBRE_APP;
pub use operaciones::Operaciones;
pub use preferencias::Almacen;

/// Carpeta de las operaciones dentro de la carpeta de datos de la app (`docs/flujos.md`).
pub const CARPETA_OPERACIONES: &str = "operaciones";

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
    let conocidas = deteccion::rutas_automaticas();
    almacen.cambiar(|p| Ok(deteccion::dar_de_alta(&raiz, &conocidas, p)))
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

/// Busca en disco el personaje que pide la interfaz entre las instalaciones conocidas (registro, agente y a mano).
fn ubicar(
    personaje: &deteccion::PersonajePedido,
    almacen: &Almacen,
) -> Result<deteccion::PersonajeUbicado, error::ErrorNucleo> {
    let mut conocidas = deteccion::rutas_automaticas();
    let preferencias = almacen.leer();
    if let Ok(preferencias) = &preferencias {
        conocidas.extend(preferencias.carpetas_manuales.iter().cloned());
    }
    // Sin preferencias legibles faltan las carpetas añadidas a mano: si no se encuentra, ese es el motivo que se dice.
    match (deteccion::ubicar(personaje, &conocidas), preferencias) {
        (Err(error::ErrorNucleo::PersonajeNoEncontrado {}), Err(motivo)) => Err(motivo),
        (resultado, _) => resultado,
    }
}

/// Cuántos ficheros y bytes tiene la carpeta de un personaje, para confirmar antes de eliminarla. Solo lee.
#[tauri::command]
async fn resumen_eliminacion(
    personaje: deteccion::PersonajePedido,
    almacen: State<'_, Almacen>,
) -> Result<operaciones::ResumenEliminacion, error::ErrorNucleo> {
    Operaciones::resumen_eliminacion(&ubicar(&personaje, &almacen)?)
}

/// Saca la carpeta de un personaje del `WTF` y la guarda como una operación con copia (spec 021). Devuelve su id.
#[tauri::command]
async fn eliminar_personaje(
    personaje: deteccion::PersonajePedido,
    almacen: State<'_, Almacen>,
    operaciones: State<'_, Operaciones>,
) -> Result<String, error::ErrorNucleo> {
    operaciones.eliminar_personaje(&ubicar(&personaje, &almacen)?, sistema::juego_en_marcha)
}

/// Termina las operaciones que se cortaron (la app se cerró o se fue la luz). La interfaz lo llama al arrancar.
#[tauri::command]
async fn recuperar_operaciones(
    operaciones: State<'_, Operaciones>,
) -> Result<operaciones::Recuperacion, error::ErrorNucleo> {
    Ok(operaciones.recuperar(sistema::juego_en_marcha))
}

/// Las operaciones de la app, de la más nueva a la más antigua, con lo que ocupan (spec 022). Solo lee.
#[tauri::command]
async fn listar_operaciones(
    operaciones: State<'_, Operaciones>,
) -> Result<operaciones::Historial, error::ErrorNucleo> {
    Ok(operaciones.listar())
}

/// Qué haría deshacer una operación, para confirmarlo. Solo lee.
#[tauri::command]
async fn plan_deshacer(
    id: String,
    operaciones: State<'_, Operaciones>,
) -> Result<operaciones::PlanDeshacer, error::ErrorNucleo> {
    operaciones.plan_deshacer(&id)
}

/// Deshace una operación si el disco sigue como en su plan (`huella`). Devuelve el id del deshacer.
#[tauri::command]
async fn deshacer_operacion(
    id: String,
    huella: String,
    operaciones: State<'_, Operaciones>,
) -> Result<String, error::ErrorNucleo> {
    operaciones.deshacer(&id, &huella, sistema::juego_en_marcha)
}

/// Restaura una importación que se cortó a mitad, cuando el usuario lo pide (spec 025).
#[tauri::command]
async fn restaurar_operacion(
    id: String,
    operaciones: State<'_, Operaciones>,
) -> Result<(), error::ErrorNucleo> {
    operaciones.restaurar_operacion(&id, sistema::juego_en_marcha)
}

/// Borra una operación terminada. `restaurada_a_mano`: el usuario confirma que ya devolvió a mano lo que faltaba.
#[tauri::command]
async fn borrar_operacion(
    id: String,
    restaurada_a_mano: bool,
    operaciones: State<'_, Operaciones>,
) -> Result<(), error::ErrorNucleo> {
    operaciones.borrar(&id, restaurada_a_mano)
}

/// Abre en el explorador la carpeta de una operación. La interfaz manda el id, nunca una ruta.
#[tauri::command]
async fn abrir_carpeta_operacion(
    id: String,
    operaciones: State<'_, Operaciones>,
) -> Result<(), error::ErrorNucleo> {
    sistema::abrir_carpeta(&operaciones.carpeta_de(&id)?)
}

/// Qué se puede exportar de un personaje ahora mismo (spec 023). Solo lee.
#[tauri::command]
async fn propuesta_exportacion(
    personaje: deteccion::PersonajePedido,
    almacen: State<'_, Almacen>,
) -> Result<exportacion::Propuesta, error::ErrorNucleo> {
    exportacion::propuesta_de(&ubicar(&personaje, &almacen)?, sistema::juego_en_marcha())
}

/// Qué pasó al exportar.
#[derive(serde::Serialize)]
#[serde(tag = "resultado", rename_all = "camelCase")]
enum Exportacion {
    /// El usuario cerró el diálogo de guardar sin elegir.
    Cancelada,
    /// Nombre del fichero guardado (sin la carpeta).
    Exportada { fichero: String },
}

/// Arma el `.wowprofile` con lo pedido y lo guarda donde elija el usuario en el diálogo de Windows, que se abre desde
/// aquí: la interfaz no elige rutas. Se valida antes de abrir el diálogo. No escribe en `WTF`.
#[tauri::command]
async fn exportar_personaje<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    personaje: deteccion::PersonajePedido,
    seleccion: Vec<String>,
    nombre: String,
    descripcion: String,
    almacen: State<'_, Almacen>,
) -> Result<Exportacion, error::ErrorNucleo> {
    let pedido = exportacion::Pedido {
        seleccion,
        nombre,
        descripcion,
    };
    let paquete = exportacion::paquete(&ubicar(&personaje, &almacen)?, &pedido)?;
    let Some(ruta) = selector_de_paquetes(&app)
        .set_file_name(exportacion::nombre_de_fichero(
            &paquete.manifiesto.perfil.nombre,
        ))
        .blocking_save_file()
        .and_then(|c| c.into_path().ok())
    else {
        return Ok(Exportacion::Cancelada);
    };
    let ruta = exportacion::con_extension(ruta);
    paquete.guardar(&ruta)?;
    let fichero = ruta
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    Ok(Exportacion::Exportada { fichero })
}

/// El diálogo de Windows para elegir un `.wowprofile`, como hijo de la ventana. Lo usan exportar (guardar) e importar
/// (abrir).
fn selector_de_paquetes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> tauri_plugin_dialog::FileDialogBuilder<R> {
    let selector = app
        .dialog()
        .file()
        .add_filter("WoW Profile", &[exportacion::EXTENSION]);
    match app.get_webview_window(VENTANA_PRINCIPAL) {
        Some(ventana) => selector.set_parent(&ventana),
        None => selector,
    }
}

/// Qué pasó al abrir un paquete.
#[derive(serde::Serialize)]
#[serde(tag = "resultado", rename_all = "camelCase")]
enum Apertura {
    /// El usuario cerró el diálogo sin elegir.
    Cancelada,
    /// En una caja: el resumen lleva el manifiesto entero y la otra variante no lleva nada.
    Abierta { resumen: Box<importacion::Resumen> },
}

/// Abre el diálogo de Windows para elegir un `.wowprofile` y lo lee con todas las comprobaciones del formato (spec 024).
/// El diálogo se abre desde aquí: la interfaz no elige rutas. No escribe nada.
#[tauri::command]
async fn abrir_paquete<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    paquetes: State<'_, PaquetesAbiertos>,
) -> Result<Apertura, error::ErrorNucleo> {
    let Some(ruta) = selector_de_paquetes(&app)
        .blocking_pick_file()
        .and_then(|c| c.into_path().ok())
    else {
        return Ok(Apertura::Cancelada);
    };
    let resumen = Box::new(paquetes.abrir(&ruta)?);
    Ok(Apertura::Abierta { resumen })
}

/// Olvida el paquete abierto.
#[tauri::command]
async fn cerrar_paquete(paquetes: State<'_, PaquetesAbiertos>) -> Result<(), error::ErrorNucleo> {
    paquetes.cerrar();
    Ok(())
}

/// El plan de importar el paquete abierto (`sha256`) en esos personajes, para la vista previa. Solo lee.
#[tauri::command]
async fn plan_importacion(
    sha256: String,
    personajes: Vec<deteccion::PersonajePedido>,
    pedido: importacion::Pedido,
    almacen: State<'_, Almacen>,
    paquetes: State<'_, PaquetesAbiertos>,
) -> Result<importacion::VistaPrevia, error::ErrorNucleo> {
    let ubicados = personajes
        .iter()
        .map(|p| ubicar(p, &almacen))
        .collect::<Result<Vec<_>, _>>()?;
    paquetes.vista_previa(&sha256, ubicados, &pedido, sistema::juego_en_marcha())
}

/// Aplica el paquete abierto (`sha256`) en esos personajes, si el plan sigue siendo el que se enseñó (`huella`): una
/// operación por personaje (spec 025). Devuelve qué pasó en cada uno.
#[tauri::command]
async fn aplicar_importacion(
    sha256: String,
    huella: String,
    personajes: Vec<deteccion::PersonajePedido>,
    pedido: importacion::Pedido,
    almacen: State<'_, Almacen>,
    paquetes: State<'_, PaquetesAbiertos>,
    operaciones: State<'_, Operaciones>,
) -> Result<Vec<importacion::ResultadoPersonaje>, error::ErrorNucleo> {
    let ubicados = personajes
        .iter()
        .map(|p| ubicar(p, &almacen))
        .collect::<Result<Vec<_>, _>>()?;
    paquetes.aplicar(
        &sha256,
        &huella,
        ubicados,
        &pedido,
        &operaciones,
        sistema::juego_en_marcha,
    )
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
        resumen_eliminacion,
        eliminar_personaje,
        recuperar_operaciones,
        listar_operaciones,
        plan_deshacer,
        deshacer_operacion,
        restaurar_operacion,
        borrar_operacion,
        abrir_carpeta_operacion,
        propuesta_exportacion,
        exportar_personaje,
        abrir_paquete,
        cerrar_paquete,
        plan_importacion,
        aplicar_importacion,
        abrir_enlace
    ])
}

/// Arranca la aplicación. Si Tauri no puede abrir la ventana, no hay nada que hacer salvo terminar con un error.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let resultado = con_comandos(tauri::Builder::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let datos = app.path().app_data_dir()?;
            app.manage(Operaciones::en(datos.join(CARPETA_OPERACIONES)));
            app.manage(Almacen::en(datos));
            app.manage(PaquetesAbiertos::default());
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(error) = resultado {
        eprintln!("{NOMBRE_APP}: no se pudo arrancar: {error}");
        std::process::exit(1);
    }
}
