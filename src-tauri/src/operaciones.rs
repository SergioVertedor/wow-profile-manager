//! Operaciones (ver `docs/arquitectura.md` y `docs/flujos.md`): el único módulo que escribe en `WTF`. Cada operación es
//! una carpeta en `<datos>/operaciones/<id>/` con su diario (`operacion.json`) y las copias de lo que había. Nada se
//! quita del `WTF` sin una copia verificada, y nada de esta carpeta se borra solo (D-04).
//!
//! Hay tres tipos: `eliminacion` (spec 021), sacar la carpeta de un personaje; `importacion` (spec 025), escribir los
//! ficheros de un perfil; y `deshacer` (spec 022), devolver lo que hizo una de las otras dos. El historial (listar,
//! borrar, abrir) vive en `historial.rs`, deshacer en `deshacer.rs` e importar en `importar.rs`.

mod deshacer;
mod historial;
mod importar;

pub use deshacer::PlanDeshacer;
pub use historial::Historial;
pub use importar::{AccionImportar, Escritura, PaqueteImportado};

use crate::deteccion::{CARPETA_ACCOUNT, CARPETA_WTF, PersonajeUbicado};
use crate::disco::subcarpetas;
use crate::error::ErrorNucleo;
use crate::escritura::escribir_atomico;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

const DIARIO: &str = "operacion.json";
const CARPETA_COPIAS: &str = "copias";
const ESQUEMA: u32 = 1;
/// El mismo límite que `docs/operacion.schema.json`. Una carpeta de personaje de Retail medida tiene 153 ficheros.
const MAXIMO_CAMBIOS: usize = 4096;
/// Intentos de crear una carpeta de operación con un id que no exista ya.
const INTENTOS_ID: u32 = 8;
/// Motivo que queda en el diario de una operación que se cortó (la app se cerró o se fue la luz).
const INTERRUMPIDA: &str = "interrumpida";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Tipo {
    Importacion,
    Deshacer,
    Eliminacion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Estado {
    Preparando,
    Copiada,
    Escribiendo,
    Completa,
    Deshecha,
    Cancelada,
    FallidaRestaurada,
    FallidaSinRestaurar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Accion {
    Crear,
    Sobrescribir,
    Fusionar,
    Restaurar,
    Borrar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Destino {
    raiz_version: PathBuf,
    producto: String,
    cuenta: String,
    carpeta_reino: String,
    carpeta_personaje: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    nombre_en_datos: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Antes {
    existia: bool,
    /// Hash, tamaño y copia: solo si existía.
    #[serde(skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    copia: Option<String>,
}

impl Antes {
    const NO_EXISTIA: Self = Self {
        existia: false,
        sha256: None,
        bytes: None,
        copia: None,
    };

    fn guardado(sha256: String, bytes: u64, copia: String) -> Self {
        Self {
            existia: true,
            sha256: Some(sha256),
            bytes: Some(bytes),
            copia: Some(copia),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Despues {
    existe: bool,
    /// Hash y tamaño: solo si existe.
    #[serde(skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes: Option<u64>,
}

impl Despues {
    const NO_EXISTE: Self = Self {
        existe: false,
        sha256: None,
        bytes: None,
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cambio {
    /// Relativa a `destino.raiz_version`, con `/`.
    ruta: String,
    accion: Accion,
    de_cuenta: bool,
    /// Solo en `importacion`: el id de la entrada del paquete de la que sale.
    #[serde(skip_serializing_if = "Option::is_none")]
    entrada: Option<String>,
    antes: Antes,
    despues: Despues,
}

/// `operacion.json` según `docs/operacion.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Diario {
    esquema: u32,
    id: String,
    tipo: Tipo,
    creada: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminada: Option<String>,
    estado: Estado,
    /// Solo en `importacion`: el perfil que se importó.
    #[serde(skip_serializing_if = "Option::is_none")]
    paquete: Option<PaqueteImportado>,
    /// Solo en `deshacer`: la operación que deshace.
    #[serde(skip_serializing_if = "Option::is_none")]
    deshace: Option<String>,
    /// Solo con `estado: deshecha`: la operación que la deshizo.
    #[serde(skip_serializing_if = "Option::is_none")]
    deshecha_por: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    destino: Destino,
    cambios: Vec<Cambio>,
}

impl Diario {
    fn terminar(&mut self, estado: Estado, error: Option<String>) {
        self.estado = estado;
        self.error = error;
        self.terminada = Some(ahora_rfc3339());
    }
}

/// Lo que la interfaz enseña antes de confirmar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumenEliminacion {
    pub ruta: PathBuf,
    pub ficheros: usize,
    pub bytes: u64,
}

/// Qué hizo la recuperación al arrancar.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recuperacion {
    /// Cortadas antes de tocar el `WTF`: pasan a `cancelada`.
    pub canceladas: u32,
    /// Cortadas a mitad: se devolvieron sus ficheros.
    pub restauradas: u32,
    /// Cortadas a mitad que no se pudieron restaurar del todo: sus copias siguen en su carpeta.
    pub sin_restaurar: u32,
    /// Cortadas a mitad que esperan: hay un `Wow.exe` en marcha.
    pub pendientes: u32,
    /// Importaciones cortadas a mitad: no se restauran solas (spec 025, decisión 4). Esperan a que el usuario las
    /// restaure, y mientras no se crea ninguna otra operación.
    pub a_medias: u32,
    /// Carpetas con un `operacion.json` que no se entiende. No se tocan.
    pub ilegibles: u32,
}

/// Las operaciones de la app, compartidas por los comandos. El cerrojo hace que dos no se pisen.
pub struct Operaciones {
    dir: PathBuf,
    cerrojo: Mutex<()>,
}

impl Operaciones {
    /// `dir`: la carpeta `operaciones` dentro de la carpeta de datos de la app.
    pub fn en(dir: PathBuf) -> Self {
        Self {
            dir,
            cerrojo: Mutex::new(()),
        }
    }

    /// Cuántos ficheros y bytes tiene la carpeta del personaje. Solo lee.
    pub fn resumen_eliminacion(
        personaje: &PersonajeUbicado,
    ) -> Result<ResumenEliminacion, ErrorNucleo> {
        let carpeta = personaje.carpeta();
        let ficheros = ficheros_a_quitar(personaje)?;
        let mut bytes = 0;
        for fichero in &ficheros {
            bytes += std::fs::metadata(fichero)
                .map_err(|e| ErrorNucleo::lectura(fichero, &e))?
                .len();
        }
        Ok(ResumenEliminacion {
            ruta: carpeta,
            ficheros: ficheros.len(),
            bytes,
        })
    }

    /// Saca la carpeta del personaje del `WTF` siguiendo el orden de escritura de `docs/flujos.md`. Devuelve el id de
    /// la operación.
    pub fn eliminar_personaje(
        &self,
        personaje: &PersonajeUbicado,
        juego_en_marcha: impl Fn() -> bool,
    ) -> Result<String, ErrorNucleo> {
        let _turno = self.cerrojo.lock().unwrap_or_else(|e| e.into_inner());
        if juego_en_marcha() {
            return Err(ErrorNucleo::JuegoAbierto {});
        }
        // Una operación que quedó a medias (al arrancar el juego estaba abierto) se termina antes: si no, el siguiente
        // arranque la restauraría encima de esta. Lo que haga se ve en su diario.
        self.recuperar_sin_cerrojo(&juego_en_marcha);
        self.sin_importacion_a_medias()?;
        let carpeta = personaje.carpeta();
        let ficheros = ficheros_a_quitar(personaje)?;
        let (id, dir) = self.nueva_carpeta()?;
        let diario = match preparar(id, personaje, &carpeta, &ficheros)
            .and_then(|diario| guardar(&dir, &diario).map(|()| diario))
        {
            Ok(diario) => diario,
            Err(error) => {
                // Aún no hay copias ni se ha tocado el `WTF`: la carpeta recién creada no guarda nada.
                std::fs::remove_dir_all(&dir).ok();
                return Err(error);
            }
        };
        ejecutar(&dir, diario, &juego_en_marcha, |diario| {
            quitar(diario)?;
            quitar_carpetas_vacias(&carpeta);
            Ok(())
        })
    }

    /// Al arrancar (`docs/flujos.md`, «Al arrancar»): las cortadas antes de tocar el `WTF` se cancelan; las cortadas a
    /// mitad se restauran, salvo con el juego abierto o si son importaciones (esas se preguntan). Un diario que no se
    /// entiende no se toca.
    pub fn recuperar(&self, juego_en_marcha: impl Fn() -> bool) -> Recuperacion {
        let _turno = self.cerrojo.lock().unwrap_or_else(|e| e.into_inner());
        self.recuperar_sin_cerrojo(&juego_en_marcha)
    }

    fn recuperar_sin_cerrojo(&self, juego_en_marcha: &impl Fn() -> bool) -> Recuperacion {
        let mut recuperacion = Recuperacion::default();
        let Ok(carpetas) = subcarpetas(&self.dir) else {
            return recuperacion;
        };
        let mut deshechas = Vec::new();
        for carpeta in carpetas {
            let Some(mut diario) = leer(&carpeta.ruta) else {
                recuperacion.ilegibles += 1;
                continue;
            };
            if let (Tipo::Deshacer, Estado::Completa, Some(original)) =
                (diario.tipo, diario.estado, &diario.deshace)
            {
                deshechas.push((original.clone(), diario.id.clone()));
            }
            match diario.estado {
                Estado::Escribiendo if diario.tipo == Tipo::Importacion => {
                    recuperacion.a_medias += 1;
                    continue;
                }
                Estado::Preparando | Estado::Copiada => {
                    diario.terminar(Estado::Cancelada, Some(INTERRUMPIDA.into()));
                    recuperacion.canceladas += 1;
                }
                Estado::Escribiendo if juego_en_marcha() => {
                    recuperacion.pendientes += 1;
                    continue;
                }
                Estado::Escribiendo => {
                    if restaurar(&carpeta.ruta, &mut diario, INTERRUMPIDA.into()) {
                        recuperacion.restauradas += 1;
                    } else {
                        recuperacion.sin_restaurar += 1;
                    }
                }
                _ => continue,
            }
            if guardar(&carpeta.ruta, &diario).is_err() {
                recuperacion.ilegibles += 1;
            }
        }
        for (original, por) in deshechas {
            // Si no se puede guardar, se vuelve a intentar en el siguiente arranque.
            self.marcar_deshecha(&original, &por).ok();
        }
        recuperacion
    }

    /// Una importación cortada a mitad bloquea cualquier otra operación: otra escritura podría cambiar sus ficheros y
    /// restaurarla ya no podría devolverlos (spec 025, decisión 13).
    fn sin_importacion_a_medias(&self) -> Result<(), ErrorNucleo> {
        // Si no se puede listar, no se sigue: podría haber una a medias que no se ve.
        let a_medias = subcarpetas(&self.dir)?
            .into_iter()
            .filter_map(|c| leer(&c.ruta).map(|d| (c.nombre, d)))
            .find(|(_, d)| d.tipo == Tipo::Importacion && d.estado == Estado::Escribiendo);
        match a_medias {
            Some((id, _)) => Err(ErrorNucleo::OperacionAMedias { id }),
            None => Ok(()),
        }
    }

    /// Restaura una importación cortada a mitad, cuando el usuario lo pide. Solo esas: el resto se restaura solo al
    /// arrancar. El diario está en disco y pudo tocarse a mano: sus rutas se comprueban otra vez (carpeta y enlaces). Si
    /// no queda todo como antes, devuelve qué falta.
    pub fn restaurar_operacion(
        &self,
        id: &str,
        juego_en_marcha: impl Fn() -> bool,
    ) -> Result<(), ErrorNucleo> {
        let _turno = self.cerrojo.lock().unwrap_or_else(|e| e.into_inner());
        if juego_en_marcha() {
            return Err(ErrorNucleo::JuegoAbierto {});
        }
        let dir = self.carpeta_de(id)?;
        let mut diario = leer(&dir).ok_or(ErrorNucleo::OperacionNoEncontrada {})?;
        if diario.tipo != Tipo::Importacion || diario.estado != Estado::Escribiendo {
            return Err(ErrorNucleo::NoRestaurable {});
        }
        comprobar_bases(&diario)?;
        let restaurada = restaurar(&dir, &mut diario, INTERRUMPIDA.into());
        guardar(&dir, &diario)?;
        if restaurada {
            Ok(())
        } else {
            Err(ErrorNucleo::OperacionFallida {
                id: diario.id,
                restaurada: false,
                detalle: diario.error.unwrap_or_default(),
            })
        }
    }

    /// La original de un `deshacer` `completa` pasa a `deshecha`, si aún no lo está: la app pudo cortarse entre los dos
    /// pasos del deshacer. Si ya no existe (se borró), no hay nada que marcar.
    fn marcar_deshecha(&self, original: &str, por: &str) -> Result<(), ErrorNucleo> {
        // El id sale de un diario en disco: solo vale si es una carpeta de la lista.
        let Ok(dir) = self.carpeta_de(original) else {
            return Ok(());
        };
        let Some(mut diario) = leer(&dir).filter(|d| d.estado == Estado::Completa) else {
            return Ok(());
        };
        diario.estado = Estado::Deshecha;
        diario.deshecha_por = Some(por.to_owned());
        guardar(&dir, &diario)
    }

    /// Crea `<dir>/<id>/` con un id nuevo. `create_dir` falla si ya existe, así que dos operaciones nunca comparten
    /// carpeta.
    fn nueva_carpeta(&self) -> Result<(String, PathBuf), ErrorNucleo> {
        std::fs::create_dir_all(&self.dir).map_err(|e| ErrorNucleo::escritura(&self.dir, &e))?;
        let mut ultimo_error = None;
        for _ in 0..INTENTOS_ID {
            let id = nuevo_id();
            let dir = self.dir.join(&id);
            match std::fs::create_dir(&dir) {
                Ok(()) => return Ok((id, dir)),
                Err(error) => ultimo_error = Some(ErrorNucleo::escritura(&dir, &error)),
            }
        }
        Err(ultimo_error.unwrap_or_else(|| ErrorNucleo::escritura(&self.dir, &"sin id libre")))
    }
}

/// Pasos 3 a 6 con el diario ya guardado en `preparando`. `escribir` es el paso 5 de cada tipo: si falla, se restaura.
fn ejecutar(
    dir: &Path,
    mut diario: Diario,
    juego_en_marcha: &impl Fn() -> bool,
    escribir: impl FnOnce(&Diario) -> Result<(), ErrorNucleo>,
) -> Result<String, ErrorNucleo> {
    if let Err(error) = copiar(dir, &diario) {
        return cancelar(dir, &mut diario, error);
    }
    diario.estado = Estado::Copiada;
    guardar(dir, &diario)?;
    // Copiar puede tardar: si el juego se abrió entretanto, se para aquí, con el `WTF` aún sin tocar.
    if juego_en_marcha() {
        return cancelar(dir, &mut diario, ErrorNucleo::JuegoAbierto {});
    }

    diario.estado = Estado::Escribiendo;
    guardar(dir, &diario)?;
    if let Err(error) = escribir(&diario) {
        let restaurada = restaurar(dir, &mut diario, error.detalle());
        guardar(dir, &diario)?;
        return Err(fallida(&diario, restaurada, &error));
    }
    diario.terminar(Estado::Completa, None);
    guardar(dir, &diario)?;
    Ok(diario.id)
}

/// Termina en `cancelada`: el `WTF` no se ha tocado. El juego abierto se devuelve tal cual, para que la interfaz lo
/// diga como siempre.
fn cancelar(dir: &Path, diario: &mut Diario, error: ErrorNucleo) -> Result<String, ErrorNucleo> {
    diario.terminar(Estado::Cancelada, Some(error.detalle()));
    guardar(dir, diario)?;
    match error {
        ErrorNucleo::JuegoAbierto {} => Err(error),
        _ => Err(fallida(diario, true, &error)),
    }
}

/// Los ficheros de la carpeta del personaje, si se pueden quitar sin salir de `WTF/Account`: la carpeta tiene que
/// estar de verdad dentro (sin enlaces en el camino) y no puede tener enlaces dentro. Seguir un enlace o una unión
/// podría borrar ficheros de fuera.
fn ficheros_a_quitar(personaje: &PersonajeUbicado) -> Result<Vec<PathBuf>, ErrorNucleo> {
    let carpeta = personaje.carpeta();
    let account = personaje
        .raiz_version
        .join(CARPETA_WTF)
        .join(CARPETA_ACCOUNT);
    let real = |ruta: &Path| {
        ruta.canonicalize()
            .map_err(|e| ErrorNucleo::lectura(ruta, &e))
    };
    if !real(&carpeta)?.starts_with(real(&account)?) || es_enlace(&carpeta) {
        return Err(enlace(&carpeta));
    }
    let ficheros = ficheros_de(&carpeta)?;
    if ficheros.len() > MAXIMO_CAMBIOS {
        return Err(ErrorNucleo::escritura(
            &carpeta,
            &format!("más de {MAXIMO_CAMBIOS} ficheros"),
        ));
    }
    Ok(ficheros)
}

/// La carpeta del personaje del destino o, si `de_cuenta`, la de su cuenta, relativa a la raíz y con `/`.
fn base_de(destino: &Destino, de_cuenta: bool) -> String {
    let cuenta = format!("{CARPETA_WTF}/{CARPETA_ACCOUNT}/{}", destino.cuenta);
    if de_cuenta {
        cuenta
    } else {
        format!(
            "{cuenta}/{}/{}",
            destino.carpeta_reino, destino.carpeta_personaje
        )
    }
}

/// Cada cambio cae dentro de su base (la carpeta del personaje o, con `deCuenta`, la de su cuenta), con nombres que
/// Windows usa tal cual, y ningún tramo de su camino que exista es un enlace o una unión. El diario puede venir del
/// disco y haberse tocado a mano; en una importación, la ruta sale de un paquete ajeno.
fn comprobar_bases(diario: &Diario) -> Result<(), ErrorNucleo> {
    let raiz = &diario.destino.raiz_version;
    for cambio in &diario.cambios {
        let base = base_de(&diario.destino, cambio.de_cuenta);
        en_disco(diario, cambio)?;
        let dentro = cambio.ruta.starts_with(&format!("{base}/"));
        if !dentro || ruta_segura(raiz, &base).is_none() {
            return Err(ErrorNucleo::escritura(
                Path::new(&cambio.ruta),
                &"ruta fuera de su carpeta",
            ));
        }
        let mut tramo = raiz.clone();
        for parte in Path::new(&cambio.ruta).components() {
            tramo.push(parte);
            if es_enlace(&tramo) {
                return Err(enlace(&tramo));
            }
        }
    }
    Ok(())
}

/// Un enlace simbólico o una unión de Windows (las dos son «sustitutos de nombre» para `is_symlink`).
pub(crate) fn es_enlace(ruta: &Path) -> bool {
    std::fs::symlink_metadata(ruta).is_ok_and(|m| m.file_type().is_symlink())
}

fn enlace(ruta: &Path) -> ErrorNucleo {
    ErrorNucleo::EnlaceNoAdmitido {
        ruta: ruta.display().to_string(),
    }
}

/// Paso 2: el diario en `preparando`, con todos los cambios y el hash de cada fichero tal como está ahora.
fn preparar(
    id: String,
    personaje: &PersonajeUbicado,
    carpeta: &Path,
    ficheros: &[PathBuf],
) -> Result<Diario, ErrorNucleo> {
    let base = personaje.ruta_relativa();
    let mut cambios = Vec::with_capacity(ficheros.len());
    for (n, fichero) in ficheros.iter().enumerate() {
        let bytes = std::fs::read(fichero).map_err(|e| ErrorNucleo::lectura(fichero, &e))?;
        cambios.push(Cambio {
            ruta: format!("{base}/{}", relativa_con_barras(carpeta, fichero)),
            accion: Accion::Borrar,
            de_cuenta: false,
            entrada: None,
            antes: Antes::guardado(sha256(&bytes), bytes.len() as u64, copia_n(n)),
            despues: Despues::NO_EXISTE,
        });
    }
    Ok(Diario {
        esquema: ESQUEMA,
        id,
        tipo: Tipo::Eliminacion,
        creada: ahora_rfc3339(),
        terminada: None,
        estado: Estado::Preparando,
        paquete: None,
        deshace: None,
        deshecha_por: None,
        error: None,
        destino: destino_de(personaje),
        cambios,
    })
}

/// El `destino` del diario de una operación sobre `personaje`.
fn destino_de(personaje: &PersonajeUbicado) -> Destino {
    Destino {
        raiz_version: personaje.raiz_version.clone(),
        producto: personaje.producto.codigo().to_owned(),
        cuenta: personaje.cuenta.clone(),
        carpeta_reino: personaje.reino.clone(),
        carpeta_personaje: personaje.personaje.clone(),
        nombre_en_datos: personaje.nombre_en_datos.clone(),
    }
}

/// Paso 3: copia cada fichero y relee la copia contra el hash de `preparar`. Si no cuadra (la copia salió mal o el
/// fichero cambió entre medias), se para sin haber tocado el `WTF`.
fn copiar(dir: &Path, diario: &Diario) -> Result<(), ErrorNucleo> {
    for cambio in &diario.cambios {
        let (Some(sha), Some(copia)) = (&cambio.antes.sha256, &cambio.antes.copia) else {
            continue;
        };
        let origen = en_disco(diario, cambio)?;
        let bytes = std::fs::read(&origen).map_err(|e| ErrorNucleo::lectura(&origen, &e))?;
        let copia = dir.join(copia);
        escribir_atomico(&copia, &bytes)?;
        comprobar(&copia, sha)?;
    }
    Ok(())
}

/// Paso 5: quita cada fichero. Para en el primer fallo.
fn quitar(diario: &Diario) -> Result<(), ErrorNucleo> {
    for cambio in &diario.cambios {
        let ruta = en_disco(diario, cambio)?;
        sigue_como_antes(&ruta, cambio)?;
        std::fs::remove_file(&ruta).map_err(|e| ErrorNucleo::escritura(&ruta, &e))?;
    }
    Ok(())
}

/// Justo antes de escribir o quitar: el fichero tiene que seguir como cuando se copió. Si otro programa (el juego, el
/// antivirus, una sincronización) lo cambió entre medias, su contenido nuevo no tiene copia: se para y se restaura.
fn sigue_como_antes(ruta: &Path, cambio: &Cambio) -> Result<(), ErrorNucleo> {
    if hash_en_disco(ruta)? == cambio.antes.sha256 {
        Ok(())
    } else {
        Err(ErrorNucleo::escritura(ruta, &"cambió desde que se copió"))
    }
}

/// Deja cada fichero como estaba antes de la operación, desde su copia verificada, sin pisar nada desconocido (ver
/// `restaurar_uno`). Deja el diario en `fallida-restaurada` o, si alguna copia no cuadra o no se puede escribir,
/// `fallida-sin-restaurar` con la lista de lo que falta. Devuelve si quedó todo restaurado.
fn restaurar(dir: &Path, diario: &mut Diario, motivo: String) -> bool {
    let mut faltan = Vec::new();
    for cambio in &diario.cambios {
        if let Err(error) = restaurar_uno(dir, diario, cambio) {
            faltan.push(error.detalle());
        }
    }
    if faltan.is_empty() {
        diario.terminar(Estado::FallidaRestaurada, Some(motivo));
        true
    } else {
        let error = format!("{motivo}. Sin restaurar: {}", faltan.join("; "));
        diario.terminar(Estado::FallidaSinRestaurar, Some(error));
        false
    }
}

/// Un fichero que ya está como `antes` se deja. Solo se toca si no está o si está como lo dejó la propia operación
/// (`despues`): entonces vuelve a `antes` (su copia, o quitarlo si no existía). Si es otro (el juego volvió a crear la
/// carpeta), no se pisa: queda sin restaurar y se dice.
fn restaurar_uno(dir: &Path, diario: &Diario, cambio: &Cambio) -> Result<(), ErrorNucleo> {
    let destino = en_disco(diario, cambio)?;
    let actual = hash_en_disco(&destino)?;
    if actual == cambio.antes.sha256 {
        return Ok(());
    }
    let lo_dejo_ella = actual.is_none() || actual == cambio.despues.sha256;
    if !lo_dejo_ella {
        return Err(ErrorNucleo::escritura(
            &destino,
            &"cambió desde la operación; no se pisa",
        ));
    }
    let (Some(sha), Some(copia)) = (&cambio.antes.sha256, &cambio.antes.copia) else {
        return std::fs::remove_file(&destino).map_err(|e| ErrorNucleo::escritura(&destino, &e));
    };
    let bytes = comprobar(&dir.join(copia), sha)?;
    escribir_atomico(&destino, &bytes)?;
    comprobar(&destino, sha).map(|_| ())
}

/// Lee `ruta` y comprueba que tiene el hash `sha`. Devuelve sus bytes.
fn comprobar(ruta: &Path, sha: &str) -> Result<Vec<u8>, ErrorNucleo> {
    let bytes = std::fs::read(ruta).map_err(|e| ErrorNucleo::lectura(ruta, &e))?;
    if sha256(&bytes) == sha {
        Ok(bytes)
    } else {
        Err(ErrorNucleo::escritura(ruta, &"la copia no cuadra"))
    }
}

/// Hash y tamaño de lo que hay en `ruta`, o `None` si no hay nada.
fn contenido(ruta: &Path) -> Result<Option<(String, u64)>, ErrorNucleo> {
    match std::fs::read(ruta) {
        Ok(bytes) => Ok(Some((sha256(&bytes), bytes.len() as u64))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ErrorNucleo::lectura(ruta, &error)),
    }
}

/// El hash de lo que hay en `ruta`, o `None` si no hay nada.
fn hash_en_disco(ruta: &Path) -> Result<Option<String>, ErrorNucleo> {
    Ok(contenido(ruta)?.map(|(hash, _)| hash))
}

/// Nombre de la copia número `n` dentro de la carpeta de la operación.
fn copia_n(n: usize) -> String {
    format!("{CARPETA_COPIAS}/{n}")
}

fn en_disco(diario: &Diario, cambio: &Cambio) -> Result<PathBuf, ErrorNucleo> {
    ruta_segura(&diario.destino.raiz_version, &cambio.ruta).ok_or_else(|| {
        ErrorNucleo::escritura(Path::new(&cambio.ruta), &"ruta fuera de WTF/Account")
    })
}

/// Une una ruta del diario a la raíz de la versión solo si es `WTF/Account/...` hecha de nombres normales: el diario
/// está en disco y podría haberse tocado a mano.
fn ruta_segura(raiz_version: &Path, relativa: &str) -> Option<PathBuf> {
    let relativa = Path::new(relativa);
    let normal = relativa.components().all(|c| match c {
        Component::Normal(nombre) => nombre.to_str().is_some_and(segmento_literal),
        _ => false,
    });
    let dentro = relativa.starts_with(Path::new(CARPETA_WTF).join(CARPETA_ACCOUNT));
    // Tramo a tramo, para que la ruta lleve los separadores del sistema y no una mezcla de `/` y `\`.
    (normal && dentro).then(|| raiz_version.join(relativa.components().collect::<PathBuf>()))
}

/// Un nombre que Windows usa tal cual (`docs/operacion.schema.json`, `segmento`): sin caracteres prohibidos (`:` abriría
/// un flujo alternativo de NTFS: `C:` a mitad de una ruta escribe en otro sitio), sin punto ni espacio al final (Win32
/// los recorta: `.. ` sería `..`) y sin nombre de dispositivo (`CON`, `NUL`, `COM1`…), con o sin extensión.
pub(crate) fn segmento_literal(nombre: &str) -> bool {
    const DISPOSITIVOS: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];
    const NUMERADOS: [&str; 2] = ["COM", "LPT"];
    const PROHIBIDOS: [char; 9] = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
    if nombre
        .chars()
        .any(|c| PROHIBIDOS.contains(&c) || c.is_control())
    {
        return false;
    }
    let base = nombre
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let dispositivo = DISPOSITIVOS.contains(&base.as_str())
        || NUMERADOS.iter().any(|p| {
            base.len() == p.len() + 1
                && base.starts_with(p)
                && base.chars().last().is_some_and(|c| c.is_ascii_digit())
        });
    !nombre.is_empty() && !nombre.ends_with(['.', ' ']) && !dispositivo
}

/// Quita las carpetas que quedaron vacías, de la más honda a la de la carpeta del personaje. Si una no se puede quitar
/// (el explorador la tiene abierta, por ejemplo), se queda: los ficheros ya tienen copia y una carpeta vacía no es
/// configuración.
fn quitar_carpetas_vacias(carpeta: &Path) {
    let mut carpetas = vec![carpeta.to_path_buf()];
    let mut i = 0;
    while let Some(actual) = carpetas.get(i).cloned() {
        if let Ok(hijas) = subcarpetas(&actual) {
            carpetas.extend(hijas.into_iter().map(|h| h.ruta));
        }
        i += 1;
    }
    for vacia in carpetas.iter().rev() {
        // `remove_dir` solo quita carpetas vacías; si falla, se queda (ver arriba).
        std::fs::remove_dir(vacia).ok();
    }
}

/// Todos los ficheros de `carpeta`, en todas sus subcarpetas, ordenados.
fn ficheros_de(carpeta: &Path) -> Result<Vec<PathBuf>, ErrorNucleo> {
    let mut pendientes = vec![carpeta.to_path_buf()];
    let mut ficheros = Vec::new();
    while let Some(actual) = pendientes.pop() {
        let entradas = std::fs::read_dir(&actual).map_err(|e| ErrorNucleo::lectura(&actual, &e))?;
        for entrada in entradas {
            let entrada = entrada.map_err(|e| ErrorNucleo::lectura(&actual, &e))?;
            let ruta = entrada.path();
            // `file_type` de la entrada no sigue enlaces, a diferencia de `is_dir`.
            let tipo = entrada
                .file_type()
                .map_err(|e| ErrorNucleo::lectura(&ruta, &e))?;
            if tipo.is_symlink() {
                return Err(enlace(&ruta));
            }
            if tipo.is_dir() {
                pendientes.push(ruta);
            } else {
                ficheros.push(ruta);
            }
        }
    }
    ficheros.sort();
    Ok(ficheros)
}

fn relativa_con_barras(carpeta: &Path, fichero: &Path) -> String {
    fichero
        .strip_prefix(carpeta)
        .unwrap_or(fichero)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn guardar(dir: &Path, diario: &Diario) -> Result<(), ErrorNucleo> {
    let ruta = dir.join(DIARIO);
    let json = serde_json::to_vec_pretty(diario).map_err(|e| ErrorNucleo::escritura(&ruta, &e))?;
    escribir_atomico(&ruta, &json)
}

fn leer(dir: &Path) -> Option<Diario> {
    let bytes = std::fs::read(dir.join(DIARIO)).ok()?;
    serde_json::from_slice::<Diario>(&bytes)
        .ok()
        .filter(|d| d.esquema == ESQUEMA)
}

fn fallida(diario: &Diario, restaurada: bool, error: &ErrorNucleo) -> ErrorNucleo {
    ErrorNucleo::OperacionFallida {
        id: diario.id.clone(),
        restaurada,
        detalle: error.detalle(),
    }
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Bits altos del sufijo del id: el milisegundo (0–999 cabe en 10).
const BITS_MILISEGUNDO: u32 = 10;
/// Bits bajos del sufijo del id: al azar.
const BITS_AZAR: u32 = 32 - BITS_MILISEGUNDO;
const MASCARA_AZAR: u64 = (1 << BITS_AZAR) - 1;

/// `AAAAMMDD-HHMMSS-<8 hex>` en UTC. Los bits altos de los 8 hex son el milisegundo, para que dos operaciones del mismo
/// segundo se ordenen por el id; los bajos salen de `RandomState`, que la biblioteca estándar siembra al azar.
fn nuevo_id() -> String {
    use std::hash::BuildHasher;

    let ahora = time::OffsetDateTime::now_utc();
    let azar =
        std::collections::hash_map::RandomState::new().hash_one(ahora.unix_timestamp_nanos());
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}-{:08x}",
        ahora.year(),
        u8::from(ahora.month()),
        ahora.day(),
        ahora.hour(),
        ahora.minute(),
        ahora.second(),
        (u64::from(ahora.millisecond()) << BITS_AZAR) | (azar & MASCARA_AZAR)
    )
}

/// RFC 3339 en UTC, a segundos (`2026-11-18T09:35:49Z`).
pub(crate) fn ahora_rfc3339() -> String {
    let ahora = time::OffsetDateTime::now_utc();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        ahora.year(),
        u8::from(ahora.month()),
        ahora.day(),
        ahora.hour(),
        ahora.minute(),
        ahora.second()
    )
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::deteccion::Producto;
    use std::fs::{create_dir_all, write};

    pub(super) const ADDONS_TXT: &str = "Prat-3.0: enabled\r\n";
    pub(super) const FLAVOR_FOREVER: &str = "Product Flavor!STRING:0\r\nwowf\r\n";
    pub(super) const PRAT_LUA: &str = "Prat3DB = {}\r\n";

    pub(super) struct Escenario {
        pub(super) _tmp: tempfile::TempDir,
        pub(super) personaje: PersonajeUbicado,
        pub(super) operaciones: Operaciones,
        pub(super) datos: PathBuf,
    }

    /// Una versión con una cuenta, un personaje con dos ficheros (uno en `SavedVariables`) y otro personaje al lado.
    pub(super) fn escenario() -> Escenario {
        let tmp = tempfile::tempdir().unwrap();
        let raiz_version = tmp.path().join("_forever_");
        let personaje = PersonajeUbicado {
            raiz_version: raiz_version.clone(),
            producto: Producto::Forever,
            cuenta: "1#1".into(),
            reino: "93".into(),
            personaje: "Goldberg-Soulreaper".into(),
            nombre_en_datos: Some("Goldberg Soulreaper".into()),
        };
        let carpeta = personaje.carpeta();
        create_dir_all(carpeta.join("SavedVariables")).unwrap();
        write(raiz_version.join(".flavor.info"), FLAVOR_FOREVER).unwrap();
        write(carpeta.join("AddOns.txt"), ADDONS_TXT).unwrap();
        write(carpeta.join("SavedVariables/Prat-3.0.lua"), PRAT_LUA).unwrap();
        let vecino = raiz_version.join("WTF/Account/1#1/93/Goldberg-Joestar");
        create_dir_all(&vecino).unwrap();
        write(vecino.join("AddOns.txt"), "x").unwrap();
        let datos = tmp.path().join("datos").join("operaciones");
        Escenario {
            operaciones: Operaciones::en(datos.clone()),
            _tmp: tmp,
            personaje,
            datos,
        }
    }

    pub(super) fn diario_de(datos: &Path, id: &str) -> Diario {
        leer(&datos.join(id)).unwrap()
    }

    #[test]
    fn saca_la_carpeta_con_copias_verificadas() {
        let e = escenario();
        let id = e
            .operaciones
            .eliminar_personaje(&e.personaje, || false)
            .unwrap();

        assert!(!e.personaje.carpeta().exists());
        let vecino = e
            .personaje
            .raiz_version
            .join("WTF/Account/1#1/93/Goldberg-Joestar");
        assert!(vecino.join("AddOns.txt").is_file(), "el vecino no se toca");
        let diario = diario_de(&e.datos, &id);
        assert_eq!(diario.estado, Estado::Completa);
        assert!(diario.terminada.is_some());
        assert_eq!(diario.cambios.len(), 2);
        assert_eq!(
            diario.cambios[1].ruta,
            "WTF/Account/1#1/93/Goldberg-Soulreaper/SavedVariables/Prat-3.0.lua"
        );
        for cambio in &diario.cambios {
            let copia = std::fs::read(e.datos.join(&id).join(cambio.antes.copia.as_ref().unwrap()))
                .unwrap();
            assert_eq!(Some(sha256(&copia)), cambio.antes.sha256);
        }
    }

    #[test]
    fn el_diario_tiene_la_forma_del_schema() {
        let e = escenario();
        let id = e
            .operaciones
            .eliminar_personaje(&e.personaje, || false)
            .unwrap();
        let bytes = std::fs::read(e.datos.join(&id).join(DIARIO)).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["tipo"], "eliminacion");
        assert_eq!(json["estado"], "completa");
        assert_eq!(json["destino"]["producto"], "wowf");
        assert_eq!(json["destino"]["carpetaPersonaje"], "Goldberg-Soulreaper");
        assert_eq!(json["cambios"][0]["accion"], "borrar");
        assert_eq!(json["cambios"][0]["deCuenta"], false);
        assert_eq!(json["cambios"][0]["despues"]["existe"], false);
        assert_eq!(json["cambios"][0]["antes"]["copia"], "copias/0");
        assert!(json.get("error").is_none());
        let partes: Vec<&str> = id.split('-').collect();
        assert_eq!(
            partes.iter().map(|p| p.len()).collect::<Vec<_>>(),
            [8, 6, 8],
            "{id}"
        );
    }

    #[test]
    fn con_el_juego_abierto_no_hace_nada() {
        let e = escenario();
        assert_eq!(
            e.operaciones.eliminar_personaje(&e.personaje, || true),
            Err(ErrorNucleo::JuegoAbierto {})
        );
        assert!(e.personaje.carpeta().join("AddOns.txt").is_file());
        assert!(
            !e.datos.exists(),
            "ni siquiera crea la carpeta de operaciones"
        );
    }

    #[test]
    fn un_fichero_que_cambia_al_copiar_para_sin_tocar_el_wtf() {
        let e = escenario();
        let carpeta = e.personaje.carpeta();
        let ficheros = ficheros_de(&carpeta).unwrap();
        let diario = preparar("x".into(), &e.personaje, &carpeta, &ficheros).unwrap();
        write(carpeta.join("AddOns.txt"), "cambiado").unwrap();

        assert!(copiar(&e.datos.join("x"), &diario).is_err());
        assert!(carpeta.join("AddOns.txt").is_file());
        assert!(carpeta.join("SavedVariables/Prat-3.0.lua").is_file());
    }

    /// Un fichero que otro programa (el antivirus, el juego) tiene abierto sin dejar borrarlo. Leerlo sí se puede.
    #[test]
    #[cfg(windows)]
    fn si_falla_quitar_un_fichero_devuelve_los_ya_quitados() {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 1;

        let e = escenario();
        let bloqueado = e.personaje.carpeta().join("SavedVariables/Prat-3.0.lua");
        let abierto = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&bloqueado)
            .unwrap();

        let resultado = e.operaciones.eliminar_personaje(&e.personaje, || false);

        drop(abierto);
        let Err(ErrorNucleo::OperacionFallida { id, restaurada, .. }) = resultado else {
            panic!("esperaba OperacionFallida: {resultado:?}");
        };
        assert!(restaurada);
        let addons = std::fs::read_to_string(e.personaje.carpeta().join("AddOns.txt")).unwrap();
        assert_eq!(addons, ADDONS_TXT);
        let diario = diario_de(&e.datos, &id);
        assert_eq!(diario.estado, Estado::FallidaRestaurada);
        assert!(diario.error.is_some());
    }

    /// Deja una operación como si la app se hubiera cortado en `estado` tras quitar solo el primer fichero.
    pub(super) fn cortada(e: &Escenario, estado: Estado) -> String {
        let id = e
            .operaciones
            .eliminar_personaje(&e.personaje, || false)
            .unwrap();
        let dir = e.datos.join(&id);
        let mut diario = diario_de(&e.datos, &id);
        diario.estado = estado;
        diario.terminada = None;
        guardar(&dir, &diario).unwrap();
        for cambio in diario.cambios.iter().skip(1) {
            let destino = en_disco(&diario, cambio).unwrap();
            create_dir_all(destino.parent().unwrap()).unwrap();
            std::fs::copy(dir.join(cambio.antes.copia.as_ref().unwrap()), destino).unwrap();
        }
        id
    }

    #[test]
    fn al_arrancar_restaura_la_cortada_a_mitad() {
        let e = escenario();
        let id = cortada(&e, Estado::Escribiendo);

        let recuperacion = e.operaciones.recuperar(|| false);

        assert_eq!(recuperacion.restauradas, 1);
        let addons = std::fs::read_to_string(e.personaje.carpeta().join("AddOns.txt")).unwrap();
        assert_eq!(addons, ADDONS_TXT);
        assert!(
            e.personaje
                .carpeta()
                .join("SavedVariables/Prat-3.0.lua")
                .is_file()
        );
        let diario = diario_de(&e.datos, &id);
        assert_eq!(diario.estado, Estado::FallidaRestaurada);
        assert_eq!(diario.error.as_deref(), Some(INTERRUMPIDA));
        assert_eq!(e.operaciones.recuperar(|| false), Recuperacion::default());
    }

    #[test]
    fn al_arrancar_con_el_juego_abierto_espera() {
        let e = escenario();
        let id = cortada(&e, Estado::Escribiendo);

        assert_eq!(e.operaciones.recuperar(|| true).pendientes, 1);
        assert!(!e.personaje.carpeta().join("AddOns.txt").exists());
        assert_eq!(diario_de(&e.datos, &id).estado, Estado::Escribiendo);
    }

    #[test]
    fn al_arrancar_cancela_las_cortadas_antes_de_escribir() {
        let e = escenario();
        let id = cortada(&e, Estado::Copiada);

        assert_eq!(e.operaciones.recuperar(|| false).canceladas, 1);
        let diario = diario_de(&e.datos, &id);
        assert_eq!(diario.estado, Estado::Cancelada);
        assert!(diario.terminada.is_some());
    }

    #[test]
    fn al_arrancar_no_toca_un_diario_ilegible() {
        let e = escenario();
        let dir = e.datos.join("20261118-093549-0000abcd");
        create_dir_all(&dir).unwrap();
        write(dir.join(DIARIO), "{ roto").unwrap();

        assert_eq!(e.operaciones.recuperar(|| false).ilegibles, 1);
        assert_eq!(std::fs::read_to_string(dir.join(DIARIO)).unwrap(), "{ roto");
    }

    #[test]
    fn un_segmento_con_caracteres_prohibidos_no_es_literal() {
        for nombre in [
            "C:", "a:b", "x*", "x?", "a\"b", "<", ">", "a|b", "a\\b", "a\tb",
        ] {
            assert!(!segmento_literal(nombre), "{nombre}");
        }
        assert!(segmento_literal("Goldberg-Soulreaper"));
        assert!(segmento_literal("1#1"));
    }

    #[test]
    fn una_ruta_del_diario_fuera_de_wtf_account_no_se_usa() {
        let raiz = Path::new("C:/WoW/_forever_");
        assert!(ruta_segura(raiz, "WTF/Account/1#1/93/P/AddOns.txt").is_some());
        assert!(ruta_segura(raiz, "WTF/Account/1#1/../../../x").is_none());
        assert!(ruta_segura(raiz, "Interface/AddOns/x.lua").is_none());
        assert!(ruta_segura(raiz, "C:/Windows/x").is_none());
        assert!(ruta_segura(raiz, "/WTF/Account/x").is_none());
        assert!(ruta_segura(raiz, "WTF/Account/1#1/.. /x").is_none());
        assert!(ruta_segura(raiz, "WTF/Account/1#1/x./y").is_none());
        assert!(ruta_segura(raiz, "WTF/Account/1#1/nul.txt").is_none());
        assert!(ruta_segura(raiz, "WTF/Account/1#1/COM3").is_none());
        assert!(ruta_segura(raiz, "WTF/Account/1#1/COMMON/Console.lua").is_some());
    }

    #[test]
    fn el_resumen_cuenta_ficheros_y_bytes() {
        let e = escenario();
        let resumen = Operaciones::resumen_eliminacion(&e.personaje).unwrap();
        assert_eq!(resumen.ficheros, 2);
        assert_eq!(resumen.bytes, (ADDONS_TXT.len() + PRAT_LUA.len()) as u64);
    }

    #[test]
    fn si_el_juego_se_abre_mientras_copia_para_sin_tocar_el_wtf() {
        let e = escenario();
        let preguntas = std::cell::Cell::new(0);
        // Cerrado al empezar (y al recuperar pendientes); abierto al terminar de copiar.
        let juego = || {
            preguntas.set(preguntas.get() + 1);
            preguntas.get() > 1
        };

        let resultado = e.operaciones.eliminar_personaje(&e.personaje, juego);

        assert_eq!(resultado, Err(ErrorNucleo::JuegoAbierto {}));
        assert!(e.personaje.carpeta().join("AddOns.txt").is_file());
        let ids = subcarpetas(&e.datos).unwrap();
        assert_eq!(
            diario_de(&e.datos, &ids[0].nombre).estado,
            Estado::Cancelada
        );
    }

    #[test]
    fn una_pendiente_se_termina_antes_de_otra_operacion() {
        let e = escenario();
        let vieja = cortada(&e, Estado::Escribiendo);

        let nueva = e
            .operaciones
            .eliminar_personaje(&e.personaje, || false)
            .unwrap();

        assert_eq!(
            diario_de(&e.datos, &vieja).estado,
            Estado::FallidaRestaurada
        );
        assert_eq!(diario_de(&e.datos, &nueva).estado, Estado::Completa);
        assert!(!e.personaje.carpeta().exists());
        assert_eq!(e.operaciones.recuperar(|| false), Recuperacion::default());
    }

    #[test]
    fn restaurar_no_pisa_un_fichero_que_cambio_ni_usa_una_copia_rota() {
        let e = escenario();
        let id = cortada(&e, Estado::Escribiendo);
        let dir = e.datos.join(&id);
        // El juego volvió a crear un fichero, y la copia del otro se estropeó.
        write(
            e.personaje.carpeta().join("SavedVariables/Prat-3.0.lua"),
            "nuevo",
        )
        .unwrap();
        write(dir.join("copias/0"), "roto").unwrap();

        let recuperacion = e.operaciones.recuperar(|| false);

        assert_eq!(recuperacion.sin_restaurar, 1);
        let prat = e.personaje.carpeta().join("SavedVariables/Prat-3.0.lua");
        assert_eq!(std::fs::read_to_string(prat).unwrap(), "nuevo");
        assert!(!e.personaje.carpeta().join("AddOns.txt").exists());
        let diario = diario_de(&e.datos, &id);
        assert_eq!(diario.estado, Estado::FallidaSinRestaurar);
        let error = diario.error.unwrap();
        assert!(
            error.contains("Prat-3.0.lua") && error.contains("copias"),
            "{error}"
        );
    }

    /// Una unión de Windows (`mklink /J`, no pide permisos de administrador) dentro de la carpeta del personaje que
    /// apunta fuera: no se sigue ni se borra nada.
    #[test]
    #[cfg(windows)]
    fn una_union_dentro_del_personaje_se_rechaza_sin_tocar_nada() {
        let e = escenario();
        let fuera = e._tmp.path().join("fuera");
        create_dir_all(&fuera).unwrap();
        write(fuera.join("valioso.txt"), "no tocar").unwrap();
        let union = e.personaje.carpeta().join("enlace");
        let creada = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&union)
            .arg(&fuera)
            .output()
            .unwrap();
        assert!(creada.status.success(), "{creada:?}");

        let resultado = e.operaciones.eliminar_personaje(&e.personaje, || false);

        assert!(
            matches!(resultado, Err(ErrorNucleo::EnlaceNoAdmitido { .. })),
            "{resultado:?}"
        );
        assert!(fuera.join("valioso.txt").is_file());
        assert!(e.personaje.carpeta().join("AddOns.txt").is_file());
        assert!(!e.datos.exists() || subcarpetas(&e.datos).unwrap().is_empty());
        std::fs::remove_dir(&union).unwrap();
    }
}
