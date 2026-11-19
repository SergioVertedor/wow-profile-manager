//! Núcleo de WoW Profile Manager. Los módulos siguen `docs/arquitectura.md`; los comandos de Tauri son la única
//! puerta de entrada desde la interfaz y no contienen lógica: delegan en módulos que se prueban sin Tauri.

mod actualizacion;
mod adaptadores;
mod addons;
mod agente_battlenet;
mod bpsv;
mod claves_personaje;
mod config_addons;
mod descubrimiento;
mod deteccion;
mod diagnostico;
mod disco;
mod enlaces;
mod error;
mod escritura;
mod exportacion;
mod idioma;
mod importacion;
mod info;
mod lua;
mod modo_edicion;
mod operaciones;
mod paquetes;
mod piezas;
mod portable;
mod preferencias;
mod sistema;
mod toc;

pub use diagnostico::Registro;
pub use importacion::PaquetesAbiertos;
pub use info::NOMBRE_APP;
pub use operaciones::Operaciones;
pub use preferencias::Almacen;

/// Carpeta de las operaciones dentro de la carpeta de datos de la app (`docs/flujos.md`).
pub const CARPETA_OPERACIONES: &str = "operaciones";

/// Etiqueta de la ventana de `tauri.conf.json`; el selector de carpetas se abre como hijo suyo.
const VENTANA_PRINCIPAL: &str = "main";
/// Evento con cada línea nueva del registro de diagnóstico (spec 029).
const EVENTO_LINEA: &str = "diagnostico://linea";
/// Evento que avisa a la interfaz de que se pidió cerrar la ventana con una operación en marcha (spec 029).
const EVENTO_CIERRE_BLOQUEADO: &str = "cierre://bloqueado";

use std::path::PathBuf;
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

/// Deja en el registro el error de un comando que falla, con su código y sus datos (spec 029).
fn con_error_registrado<T>(
    comando: &str,
    resultado: Result<T, error::ErrorNucleo>,
) -> Result<T, error::ErrorNucleo> {
    if let Err(error) = &resultado {
        log::warn!(
            "{comando}: {}",
            serde_json::to_string(error).unwrap_or_default()
        );
    }
    resultado
}

/// Deja en el registro el resultado de un comando que escribe: que terminó bien o su error.
fn con_resultado_registrado<T>(
    comando: &str,
    resultado: Result<T, error::ErrorNucleo>,
) -> Result<T, error::ErrorNucleo> {
    if resultado.is_ok() {
        log::info!("{comando}: hecho");
    }
    con_error_registrado(comando, resultado)
}

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
    con_error_registrado(
        "detectar_instalaciones",
        deteccion::detectar_en_el_sistema(almacen.leer()),
    )
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
    con_resultado_registrado(
        "anadir_carpeta",
        almacen.cambiar(|p| Ok(deteccion::dar_de_alta(&raiz, &conocidas, p))),
    )
}

/// Olvida una carpeta añadida a mano, con su nombre y los de sus cuentas. No toca la carpeta.
#[tauri::command]
async fn quitar_carpeta(
    raiz: PathBuf,
    almacen: State<'_, Almacen>,
) -> Result<(), error::ErrorNucleo> {
    con_resultado_registrado(
        "quitar_carpeta",
        almacen.cambiar(|p| {
            p.quitar_carpeta(&raiz);
            Ok(())
        }),
    )
}

/// Pone, cambia o borra (texto vacío) el nombre propio de una instalación.
#[tauri::command]
async fn nombrar_instalacion(
    raiz: PathBuf,
    nombre: String,
    almacen: State<'_, Almacen>,
) -> Result<(), error::ErrorNucleo> {
    let cambio = preferencias::nombre_valido(&nombre).and_then(|nombre| {
        almacen.cambiar(|p| {
            p.nombrar_instalacion(&raiz, nombre);
            Ok(())
        })
    });
    con_resultado_registrado("nombrar_instalacion", cambio)
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
    let cambio = preferencias::nombre_valido(&nombre).and_then(|nombre| {
        almacen.cambiar(|p| {
            p.nombrar_cuenta(&raiz, &version, &cuenta, nombre);
            Ok(())
        })
    });
    con_resultado_registrado("nombrar_cuenta", cambio)
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
    con_error_registrado(
        "resumen_eliminacion",
        ubicar(&personaje, &almacen).and_then(|p| Operaciones::resumen_eliminacion(&p)),
    )
}

/// Saca la carpeta de un personaje del `WTF` y la guarda como una operación con copia (spec 021). Devuelve su id.
#[tauri::command]
async fn eliminar_personaje(
    personaje: deteccion::PersonajePedido,
    almacen: State<'_, Almacen>,
    operaciones: State<'_, Operaciones>,
) -> Result<String, error::ErrorNucleo> {
    let resultado = ubicar(&personaje, &almacen)
        .and_then(|p| operaciones.eliminar_personaje(&p, sistema::juego_en_marcha));
    con_resultado_registrado("eliminar_personaje", resultado)
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
    con_error_registrado("plan_deshacer", operaciones.plan_deshacer(&id))
}

/// Deshace una operación si el disco sigue como en su plan (`huella`). Devuelve el id del deshacer.
#[tauri::command]
async fn deshacer_operacion(
    id: String,
    huella: String,
    operaciones: State<'_, Operaciones>,
) -> Result<String, error::ErrorNucleo> {
    con_resultado_registrado(
        "deshacer_operacion",
        operaciones.deshacer(&id, &huella, sistema::juego_en_marcha),
    )
}

/// Restaura una importación que se cortó a mitad, cuando el usuario lo pide (spec 025).
#[tauri::command]
async fn restaurar_operacion(
    id: String,
    operaciones: State<'_, Operaciones>,
) -> Result<(), error::ErrorNucleo> {
    con_resultado_registrado(
        "restaurar_operacion",
        operaciones.restaurar_operacion(&id, sistema::juego_en_marcha),
    )
}

/// Borra una operación terminada. `restaurada_a_mano`: el usuario confirma que ya devolvió a mano lo que faltaba.
#[tauri::command]
async fn borrar_operacion(
    id: String,
    restaurada_a_mano: bool,
    operaciones: State<'_, Operaciones>,
) -> Result<(), error::ErrorNucleo> {
    con_resultado_registrado(
        "borrar_operacion",
        operaciones.borrar(&id, restaurada_a_mano),
    )
}

/// Abre en el explorador la carpeta de una operación. La interfaz manda el id, nunca una ruta.
#[tauri::command]
async fn abrir_carpeta_operacion(
    id: String,
    operaciones: State<'_, Operaciones>,
) -> Result<(), error::ErrorNucleo> {
    con_error_registrado(
        "abrir_carpeta_operacion",
        operaciones
            .carpeta_de(&id)
            .and_then(|c| sistema::abrir_carpeta(&c)),
    )
}

/// Abre en el explorador la carpeta de un personaje (spec 026). Con un enlace en el camino no se abre nada.
#[tauri::command]
async fn abrir_carpeta_personaje(
    personaje: deteccion::PersonajePedido,
    almacen: State<'_, Almacen>,
) -> Result<(), error::ErrorNucleo> {
    let resultado = ubicar(&personaje, &almacen).and_then(|ubicado| {
        ubicado.comprobar_sin_enlaces()?;
        sistema::abrir_carpeta(&ubicado.carpeta())
    });
    con_error_registrado("abrir_carpeta_personaje", resultado)
}

/// Qué se puede exportar de un personaje ahora mismo (spec 023). Solo lee.
#[tauri::command]
async fn propuesta_exportacion(
    personaje: deteccion::PersonajePedido,
    almacen: State<'_, Almacen>,
) -> Result<exportacion::Propuesta, error::ErrorNucleo> {
    let resultado = ubicar(&personaje, &almacen)
        .and_then(|p| exportacion::propuesta_de(&p, sistema::juego_en_marcha()));
    con_error_registrado("propuesta_exportacion", resultado)
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
    con_resultado_registrado(
        "exportar_personaje",
        exportar(&app, &personaje, &pedido, &almacen),
    )
}

fn exportar<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    personaje: &deteccion::PersonajePedido,
    pedido: &exportacion::Pedido,
    almacen: &Almacen,
) -> Result<Exportacion, error::ErrorNucleo> {
    let paquete = exportacion::paquete(&ubicar(personaje, almacen)?, pedido)?;
    let Some(ruta) = selector_de_paquetes(app)
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
    let resumen = con_error_registrado("abrir_paquete", paquetes.abrir(&ruta))?;
    Ok(Apertura::Abierta {
        resumen: Box::new(resumen),
    })
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
    let resultado = ubicar_todos(&personajes, &almacen).and_then(|ubicados| {
        paquetes.vista_previa(&sha256, ubicados, &pedido, sistema::juego_en_marcha())
    });
    con_error_registrado("plan_importacion", resultado)
}

fn ubicar_todos(
    personajes: &[deteccion::PersonajePedido],
    almacen: &Almacen,
) -> Result<Vec<deteccion::PersonajeUbicado>, error::ErrorNucleo> {
    personajes.iter().map(|p| ubicar(p, almacen)).collect()
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
    let resultado = ubicar_todos(&personajes, &almacen).and_then(|ubicados| {
        paquetes.aplicar(
            &sha256,
            &huella,
            ubicados,
            &pedido,
            &operaciones,
            sistema::juego_en_marcha,
        )
    });
    con_resultado_registrado("aplicar_importacion", resultado)
}

/// Abre en el navegador uno de los enlaces conocidos. La interfaz no puede pasar una URL: solo el nombre del enlace.
#[tauri::command]
fn abrir_enlace(enlace: enlaces::Enlace) -> Result<(), error::ErrorNucleo> {
    con_error_registrado("abrir_enlace", enlaces::abrir(enlace))
}

/// Lo que enseña la pestaña Diagnóstico al abrirse (spec 029).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct EstadoDiagnostico {
    /// El interruptor «Guardar registro de diagnóstico».
    guarda: bool,
    /// Las líneas de la sesión que siguen en memoria; las nuevas llegan con el evento `diagnostico://linea`.
    lineas: Vec<diagnostico::Linea>,
    /// Cuántas guarda el núcleo: la interfaz no enseña más.
    maximo: usize,
}

/// El estado del registro de diagnóstico. Solo lee.
#[tauri::command]
fn estado_diagnostico(registro: State<'_, &'static Registro>) -> EstadoDiagnostico {
    EstadoDiagnostico {
        guarda: registro.guarda(),
        lineas: registro.lineas(),
        maximo: diagnostico::MAXIMO_LINEAS,
    }
}

/// Enciende o apaga el `.txt` del registro y lo recuerda en las preferencias.
#[tauri::command]
async fn cambiar_registro(
    guardar: bool,
    almacen: State<'_, Almacen>,
    registro: State<'_, &'static Registro>,
) -> Result<(), error::ErrorNucleo> {
    let resultado = almacen.cambiar(|p| {
        p.registro = guardar;
        Ok(())
    });
    // La línea del cambio queda en el `.txt` en los dos sentidos: al encender, después de encender; al apagar, antes.
    if resultado.is_ok() && guardar {
        registro.cambiar(true);
        log::info!("guardar el registro: sí");
    } else if resultado.is_ok() {
        log::info!("guardar el registro: no");
        registro.cambiar(false);
    }
    con_error_registrado("cambiar_registro", resultado)
}

/// Abre en el explorador la carpeta de los registros. La interfaz solo lo ofrece con el registro encendido.
#[tauri::command]
async fn abrir_carpeta_registros(
    registro: State<'_, &'static Registro>,
) -> Result<(), error::ErrorNucleo> {
    con_error_registrado(
        "abrir_carpeta_registros",
        sistema::abrir_carpeta(registro.carpeta()),
    )
}

/// El interruptor «Buscar actualizaciones al arrancar». Si las preferencias no se pueden leer, apagado: sin saber lo
/// que eligió el usuario no se sale a la red.
fn actualizaciones_encendidas(almacen: &Almacen) -> bool {
    almacen.leer().is_ok_and(|p| !p.actualizaciones_apagadas)
}

#[tauri::command]
fn estado_actualizaciones(almacen: State<'_, Almacen>) -> bool {
    actualizaciones_encendidas(&almacen)
}

#[tauri::command]
async fn cambiar_actualizaciones(
    encendidas: bool,
    almacen: State<'_, Almacen>,
) -> Result<(), error::ErrorNucleo> {
    log::info!("buscar actualizaciones al arrancar: {encendidas}");
    con_error_registrado(
        "cambiar_actualizaciones",
        almacen.cambiar(|p| {
            p.actualizaciones_apagadas = !encendidas;
            Ok(())
        }),
    )
}

/// La versión publicada más nueva que esta, si la hay. La interfaz lo pide al arrancar; con el interruptor apagado o
/// en desarrollo (`tauri dev`) no se sale a la red. Sin conexión no es un error que enseñar: solo se registra.
#[tauri::command]
async fn buscar_actualizacion(
    almacen: State<'_, Almacen>,
) -> Result<Option<actualizacion::Disponible>, error::ErrorNucleo> {
    if cfg!(debug_assertions) || !actualizaciones_encendidas(&almacen) {
        return Ok(None);
    }
    match actualizacion::buscar(version_actual()) {
        Ok(disponible) => {
            if let Some(d) = &disponible {
                log::info!("versión nueva publicada: {}", d.version);
            }
            Ok(disponible)
        }
        Err(error) => {
            log::warn!("no se pudo buscar una actualización: {error:?}");
            Ok(None)
        }
    }
}

/// Descarga la versión nueva, comprueba su firma, sustituye el `.exe` y reinicia. La release se vuelve a pedir aquí:
/// la interfaz no manda ninguna URL.
#[tauri::command]
async fn instalar_actualizacion<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    almacen: State<'_, Almacen>,
) -> Result<(), error::ErrorNucleo> {
    let resultado = actualizacion::buscar(version_actual()).and_then(|disponible| {
        let disponible = disponible.ok_or(error::ErrorNucleo::ActualizacionNoDisponible {})?;
        actualizacion::instalar(&disponible, almacen.carpeta())?;
        log::info!("actualizado a {}: se reinicia", disponible.version);
        Ok(())
    });
    con_error_registrado("instalar_actualizacion", resultado)?;
    app.restart()
}

fn version_actual() -> actualizacion::Version {
    actualizacion::Version::leer(env!("CARGO_PKG_VERSION"))
        .unwrap_or(actualizacion::Version::MINIMA)
}

/// Cierra la ventana aunque haya una operación en marcha: el usuario lo ha confirmado. Lo cortado se termina al
/// volver a arrancar, como si se hubiera ido la luz (`docs/flujos.md`).
#[tauri::command]
fn cerrar_ventana<R: tauri::Runtime>(ventana: tauri::WebviewWindow<R>) {
    log::warn!("se cierra la ventana con una operación en marcha");
    if let Err(error) = ventana.destroy() {
        log::error!("no se pudo cerrar la ventana: {error}");
    }
}

/// Con una operación escribiendo en el `WTF`, cerrar la ventana no la cierra: se avisa a la interfaz, que pregunta.
fn al_pedir_cierre<R: tauri::Runtime>(ventana: &tauri::Window<R>, evento: &tauri::WindowEvent) {
    let tauri::WindowEvent::CloseRequested { api, .. } = evento else {
        return;
    };
    // `try_state`: si el cierre llegara antes de terminar `setup`, no hay operaciones y se cierra sin preguntar.
    if ventana
        .try_state::<Operaciones>()
        .is_some_and(|operaciones| operaciones.ocupada())
    {
        api.prevent_close();
        log::info!("cierre pedido con una operación en marcha: se pregunta");
        if let Err(error) = ventana.emit(EVENTO_CIERRE_BLOQUEADO, ()) {
            log::error!("no se pudo avisar del cierre: {error}");
        }
    }
}

/// Instala el registro de diagnóstico con el interruptor guardado y lo engancha a la interfaz.
fn instalar_registro<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    datos: &std::path::Path,
    almacen: &Almacen,
) -> &'static Registro {
    let guardar = almacen.leer().map(|p| p.registro).unwrap_or(false);
    let registro = diagnostico::instalar(Registro::en(
        datos.join(diagnostico::CARPETA_REGISTROS),
        guardar,
    ));
    let app = app.clone();
    registro.conectar(Box::new(move |linea| {
        // Sin ventana todavía no hay a quién avisar: la línea ya está en memoria y llega con la carga inicial.
        let _ = app.emit(EVENTO_LINEA, linea);
    }));
    log::info!(
        "{NOMBRE_APP} {} arranca; guardar el registro: {guardar}",
        env!("CARGO_PKG_VERSION")
    );
    registro
}

/// Abre la ventana principal de `tauri.conf.json` (que lleva `"create": false`) con los datos de WebView2 dentro de la
/// carpeta de datos portable: así WebView2 tampoco escribe en `%LOCALAPPDATA%`.
fn abrir_ventana(app: &tauri::App, datos: &std::path::Path) -> tauri::Result<()> {
    let config = app
        .config()
        .app
        .windows
        .first()
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    tauri::WebviewWindowBuilder::from_config(app, config)?
        .data_directory(datos.join(portable::CARPETA_WEBVIEW))
        .build()?;
    Ok(())
}

/// Sin carpeta de datos la app no puede guardar nada: lo dice con un mensaje nativo, sin abrir la ventana, y se cierra
/// al aceptarlo. `show` y no `blocking_show`: `setup` corre en el hilo principal.
fn avisar_sin_carpeta_y_salir(
    app: &tauri::AppHandle,
    datos: &std::path::Path,
    error: &std::io::Error,
) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
    eprintln!(
        "{NOMBRE_APP}: carpeta de datos no utilizable {}: {error}",
        datos.display()
    );
    let app_al_cerrar = app.clone();
    app.dialog()
        .message(portable::mensaje_sin_carpeta(
            idioma::idioma_sistema(),
            datos,
            error,
        ))
        .title(NOMBRE_APP)
        .kind(MessageDialogKind::Error)
        .show(move |_| app_al_cerrar.exit(1));
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
        abrir_carpeta_personaje,
        propuesta_exportacion,
        exportar_personaje,
        abrir_paquete,
        cerrar_paquete,
        plan_importacion,
        aplicar_importacion,
        abrir_enlace,
        estado_diagnostico,
        cambiar_registro,
        abrir_carpeta_registros,
        estado_actualizaciones,
        cambiar_actualizaciones,
        buscar_actualizacion,
        instalar_actualizacion,
        cerrar_ventana
    ])
}

/// Arranca la aplicación. Si Tauri no puede abrir la ventana, no hay nada que hacer salvo terminar con un error.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let resultado = con_comandos(tauri::Builder::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let datos = portable::carpeta_junto_a(&std::env::current_exe()?);
            if let Err(error) = portable::preparar(&datos) {
                avisar_sin_carpeta_y_salir(app.handle(), &datos, &error);
                return Ok(());
            }
            let almacen = Almacen::en(datos.clone());
            app.manage(instalar_registro(app.handle(), &datos, &almacen));
            app.manage(Operaciones::en(datos.join(CARPETA_OPERACIONES)));
            app.manage(almacen);
            app.manage(PaquetesAbiertos::default());
            abrir_ventana(app, &datos)?;
            Ok(())
        })
        .on_window_event(al_pedir_cierre)
        .run(tauri::generate_context!());
    if let Err(error) = resultado {
        eprintln!("{NOMBRE_APP}: no se pudo arrancar: {error}");
        std::process::exit(1);
    }
}
