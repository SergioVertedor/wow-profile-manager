//! Carpetas de addons que instala una importación (#73). Van aparte de los `cambios` del diario: una carpeta son
//! muchos ficheros que no existían, así que no hay nada que copiar antes. El diario guarda la lista de ficheros con su
//! hash; con ella se quita lo instalado (al fallar, al recuperar o al deshacer) sin tocar lo que alguien cambió después.
//!
//! - **Instalar** extrae la pieza en una carpeta temporal dentro de `Interface/AddOns` y la renombra al final: el juego
//!   nunca ve un addon a medias. Justo antes de renombrar se vuelve a mirar que no exista. Medido en Windows: renombrar
//!   encima de una carpeta con contenido falla, pero encima de una vacía la sustituye.
//! - **Quitar** borra los ficheros que siguen como se instalaron y las carpetas que quedan vacías. Devuelve lo que se
//!   queda.

use super::{segmento_literal, sha256};
use crate::addon_zip::{FicheroAddon, ruta_relativa_valida};
use crate::deteccion::{CARPETA_ADDONS, CARPETA_INTERFACE};
use crate::disco::es_enlace;
use crate::error::ErrorNucleo;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

/// La temporal no tiene un `.toc` con su nombre: el juego no la carga (`addons.rs`, `nombres_de_toc`).
const PREFIJO_TEMPORAL: &str = ".wowprofile-";

/// Lo que dice el diario de una carpeta instalada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CarpetaInstalada {
    /// Relativa a `destino.raiz_version`: `Interface/AddOns/<carpeta>`.
    pub ruta: String,
    /// El id de la entrada del paquete de la que sale.
    pub entrada: String,
    pub ficheros: Vec<FicheroInstalado>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FicheroInstalado {
    /// Relativa a la carpeta del addon, con `/`.
    pub ruta: String,
    pub sha256: String,
}

/// Un addon que instalar, sacado del plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalarCarpeta {
    /// Nombre de la carpeta en `Interface/AddOns`.
    pub carpeta: String,
    /// El id de la entrada del paquete.
    pub entrada: String,
}

/// Lee del paquete los ficheros de una entrada de addon, ya comprobados. Se llama al preparar y otra vez al instalar:
/// así nunca están todos los addons en memoria.
pub type Cargar<'a> = dyn Fn(&str) -> Result<Vec<FicheroAddon>, ErrorNucleo> + 'a;

/// El diario de cada carpeta: tiene que caer en `Interface/AddOns`, sin enlaces en el camino, no existir (sin
/// distinguir mayúsculas) y no repetirse. Si alguna ya existe, el plan caducó.
pub(super) fn preparar(
    raiz: &Path,
    carpetas: Vec<InstalarCarpeta>,
    cargar: &Cargar,
) -> Result<Vec<CarpetaInstalada>, ErrorNucleo> {
    let addons = carpeta_addons(raiz)?;
    let mut vistas = HashSet::new();
    let mut preparadas = Vec::with_capacity(carpetas.len());
    for carpeta in carpetas {
        if !segmento_literal(&carpeta.carpeta) || !vistas.insert(carpeta.carpeta.to_lowercase()) {
            return Err(fuera(&carpeta.carpeta));
        }
        if existe_sin_mayusculas(&addons, &carpeta.carpeta)? {
            return Err(ErrorNucleo::PlanCaducado {});
        }
        let ficheros = cargar(&carpeta.entrada)?
            .iter()
            .map(|f| FicheroInstalado {
                ruta: f.ruta.clone(),
                sha256: sha256(&f.contenido),
            })
            .collect();
        preparadas.push(CarpetaInstalada {
            ruta: format!("{CARPETA_INTERFACE}/{CARPETA_ADDONS}/{}", carpeta.carpeta),
            entrada: carpeta.entrada,
            ficheros,
        });
    }
    Ok(preparadas)
}

/// Instala cada carpeta: la extrae en una temporal y la renombra. Los ficheros tienen que ser los del diario. Para en
/// el primer fallo; quien llama restaura (`quitar`).
pub(super) fn instalar(
    raiz: &Path,
    operacion: &str,
    carpetas: &[CarpetaInstalada],
    cargar: &Cargar,
) -> Result<(), ErrorNucleo> {
    for (n, carpeta) in carpetas.iter().enumerate() {
        let destino = en_disco(raiz, &carpeta.ruta)?;
        let temporal = temporal(&destino, operacion, n);
        let ficheros = cargar(&carpeta.entrada)?;
        if !mismos_ficheros(&ficheros, &carpeta.ficheros) {
            return Err(ErrorNucleo::PlanCaducado {});
        }
        // `Interface/AddOns` puede no existir aún (una instalación sin addons); la temporal, en cambio, es nueva.
        if let Some(addons) = temporal.parent() {
            std::fs::create_dir_all(addons).map_err(|e| ErrorNucleo::escritura(addons, &e))?;
        }
        std::fs::create_dir(&temporal).map_err(|e| ErrorNucleo::escritura(&temporal, &e))?;
        for fichero in &ficheros {
            escribir_nuevo(&temporal, fichero)?;
        }
        // Si la carpeta apareció entre medias, no se pisa: con contenido, renombrar fallaría igual.
        let addons = carpeta_addons(raiz)?;
        let nombre = destino.file_name().unwrap_or_default().to_string_lossy();
        if existe_sin_mayusculas(&addons, &nombre)? {
            return Err(ErrorNucleo::PlanCaducado {});
        }
        std::fs::rename(&temporal, &destino).map_err(|e| ErrorNucleo::escritura(&destino, &e))?;
    }
    Ok(())
}

/// Quita lo que instaló cada carpeta (y lo que quedara en su temporal) si sigue como se instaló, y las carpetas que
/// quedan vacías. Devuelve, con su ruta en disco, lo que se queda: cambió desde entonces o no se pudo quitar.
pub(super) fn quitar(raiz: &Path, operacion: &str, carpetas: &[CarpetaInstalada]) -> Vec<String> {
    let mut quedan = Vec::new();
    for (n, carpeta) in carpetas.iter().enumerate() {
        let Ok(destino) = en_disco(raiz, &carpeta.ruta) else {
            quedan.push(carpeta.ruta.clone());
            continue;
        };
        // La temporal es de la operación por su nombre; la carpeta del addon, solo si tiene algo que se instaló.
        quedan.extend(quitar_de(
            &temporal(&destino, operacion, n),
            &carpeta.ficheros,
            true,
        ));
        quedan.extend(quitar_de(&destino, &carpeta.ficheros, false));
    }
    quedan
}

/// Lo que se quedará al deshacer, con su ruta en disco, para avisar antes: ficheros que ya no están como se instalaron,
/// los que se añadieron después y, si la carpeta no se puede recorrer (un enlace dentro, por ejemplo), la carpeta.
pub(super) fn cambiados(raiz: &Path, carpetas: &[CarpetaInstalada]) -> Vec<String> {
    let mut quedan = Vec::new();
    for carpeta in carpetas {
        let Ok(destino) = en_disco(raiz, &carpeta.ruta) else {
            quedan.push(carpeta.ruta.clone());
            continue;
        };
        if !destino.exists() {
            continue;
        }
        let Ok(actuales) = super::ficheros_de(&destino) else {
            quedan.push(destino.display().to_string());
            continue;
        };
        let instalados: std::collections::HashMap<PathBuf, &str> = carpeta
            .ficheros
            .iter()
            .filter_map(|f| Some((ruta_de_fichero(&destino, &f.ruta)?, f.sha256.as_str())))
            .collect();
        for actual in actuales {
            let intacto = instalados.get(&actual).is_some_and(|sha| {
                std::fs::read(&actual).is_ok_and(|bytes| sha256(&bytes) == *sha)
            });
            if !intacto {
                quedan.push(actual.display().to_string());
            }
        }
    }
    quedan
}

/// Quita de `base` los ficheros que siguen como se instalaron. Las carpetas vacías se quitan si `base` es de la
/// operación o si se quitó algo de ella: una carpeta vacía del jugador que apareció entre medias no se toca.
fn quitar_de(base: &Path, ficheros: &[FicheroInstalado], propia: bool) -> Vec<String> {
    if !base.exists() || es_enlace(base) {
        return Vec::new();
    }
    let mut quedan = Vec::new();
    let mut quitados = 0;
    for fichero in ficheros {
        let Some(ruta) = ruta_de_fichero(base, &fichero.ruta).filter(|r| sin_enlaces(base, r))
        else {
            quedan.push(format!("{}/{}", base.display(), fichero.ruta));
            continue;
        };
        match std::fs::read(&ruta) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Ok(actual) if sha256(&actual) == fichero.sha256 => match std::fs::remove_file(&ruta) {
                Ok(()) => quitados += 1,
                Err(_) => quedan.push(ruta.display().to_string()),
            },
            _ => quedan.push(ruta.display().to_string()),
        }
    }
    if propia || quitados > 0 {
        super::quitar_carpetas_vacias(base);
    }
    quedan
}

/// `Interface/AddOns` de la versión, comprobando que ningún tramo es un enlace o una unión.
fn carpeta_addons(raiz: &Path) -> Result<PathBuf, ErrorNucleo> {
    let mut tramo = raiz.to_path_buf();
    for parte in [CARPETA_INTERFACE, CARPETA_ADDONS] {
        tramo.push(parte);
        if es_enlace(&tramo) {
            return Err(super::enlace(&tramo));
        }
    }
    Ok(tramo)
}

/// La carpeta de un addon en disco, si la ruta del diario es `Interface/AddOns/<nombre>` con un nombre que Windows usa
/// tal cual, y sin enlaces en el camino. El diario está en disco y podría haberse tocado a mano.
fn en_disco(raiz: &Path, ruta: &str) -> Result<PathBuf, ErrorNucleo> {
    let prefijo = format!("{CARPETA_INTERFACE}/{CARPETA_ADDONS}/");
    let nombre = ruta
        .strip_prefix(&prefijo)
        .filter(|n| segmento_literal(n))
        .ok_or_else(|| fuera(ruta))?;
    let carpeta = carpeta_addons(raiz)?.join(nombre);
    if es_enlace(&carpeta) {
        return Err(super::enlace(&carpeta));
    }
    Ok(carpeta)
}

/// Un fichero de la lista, dentro de `base`, si su ruta relativa es segura (como en el ZIP del addon).
fn ruta_de_fichero(base: &Path, relativa: &str) -> Option<PathBuf> {
    ruta_relativa_valida(relativa).then(|| {
        let mut ruta = base.to_path_buf();
        ruta.extend(relativa.split('/'));
        ruta
    })
}

/// Ningún tramo de `ruta` por debajo de `base` es un enlace o una unión: seguirlo leería o borraría fuera del addon.
fn sin_enlaces(base: &Path, ruta: &Path) -> bool {
    let Ok(relativa) = ruta.strip_prefix(base) else {
        return false;
    };
    let mut tramo = base.to_path_buf();
    relativa.components().all(|parte| {
        tramo.push(parte);
        !es_enlace(&tramo)
    })
}

fn temporal(destino: &Path, operacion: &str, n: usize) -> PathBuf {
    destino.with_file_name(format!("{PREFIJO_TEMPORAL}{operacion}-{n}"))
}

/// Si en `addons` hay algo que se llame `nombre`, sin distinguir mayúsculas (Windows no las distingue).
fn existe_sin_mayusculas(addons: &Path, nombre: &str) -> Result<bool, ErrorNucleo> {
    let entradas = match std::fs::read_dir(addons) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(ErrorNucleo::lectura(addons, &e)),
        Ok(entradas) => entradas,
    };
    let buscado = nombre.to_lowercase();
    for entrada in entradas {
        let entrada = entrada.map_err(|e| ErrorNucleo::lectura(addons, &e))?;
        if entrada.file_name().to_string_lossy().to_lowercase() == buscado {
            return Ok(true);
        }
    }
    Ok(false)
}

fn mismos_ficheros(ficheros: &[FicheroAddon], instalados: &[FicheroInstalado]) -> bool {
    ficheros.len() == instalados.len()
        && ficheros
            .iter()
            .zip(instalados)
            .all(|(f, i)| f.ruta == i.ruta && sha256(&f.contenido) == i.sha256)
}

/// Escribe un fichero que no puede existir todavía (`create_new`), con sus carpetas.
fn escribir_nuevo(base: &Path, fichero: &FicheroAddon) -> Result<(), ErrorNucleo> {
    let ruta = ruta_de_fichero(base, &fichero.ruta).ok_or_else(|| fuera(&fichero.ruta))?;
    if let Some(padre) = ruta.parent() {
        std::fs::create_dir_all(padre).map_err(|e| ErrorNucleo::escritura(padre, &e))?;
    }
    // Otro programa pudo poner una unión dentro de la temporal entre medias.
    if !sin_enlaces(base, &ruta) {
        return Err(super::enlace(&ruta));
    }
    std::fs::File::create_new(&ruta)
        .and_then(|mut f| f.write_all(&fichero.contenido).and_then(|()| f.sync_all()))
        .map_err(|e| ErrorNucleo::escritura(&ruta, &e))
}

fn fuera(ruta: &str) -> ErrorNucleo {
    ErrorNucleo::escritura(Path::new(ruta), &"ruta fuera de Interface/AddOns")
}

#[cfg(test)]
mod tests;
