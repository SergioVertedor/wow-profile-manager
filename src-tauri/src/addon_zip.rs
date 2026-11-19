//! La carpeta de un addon como pieza del paquete (spec 036, #73): un ZIP con sus ficheros, con rutas relativas a la
//! carpeta. Empaquetar no sigue enlaces; leer comprueba cada nombre y los límites antes de devolver nada, porque lo que
//! sale de aquí se escribe en `Interface/AddOns` del que importa.

use crate::operaciones::segmento_literal;
use std::fmt;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Un addon más grande no viaja como código: solo con su enlace. Medido: 61 de 63 carpetas reales caben.
pub const MAX_BYTES_ADDON: u64 = 64 * 1024 * 1024;
/// Medido: la carpeta real con más ficheros tiene 3.083.
pub const MAX_FICHEROS_ADDON: usize = 4096;
/// Profundidad de carpetas dentro del addon. Ninguno real se acerca; corta recorridos absurdos.
const MAX_PROFUNDIDAD: usize = 32;

/// Por qué una carpeta no se empaqueta o un ZIP de addon no se acepta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorAddon {
    DemasiadoGrande,
    DemasiadosFicheros,
    /// Un nombre que no se puede escribir de forma segura en Windows (o que sale de la carpeta).
    RutaNoValida(String),
    /// Un enlace o una unión: no se sigue al empaquetar ni se acepta al leer.
    Enlace(String),
    /// No es un ZIP, o una entrada no se puede leer.
    NoSeLee(String),
}

impl fmt::Display for ErrorAddon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DemasiadoGrande => write!(f, "pasa de {MAX_BYTES_ADDON} bytes"),
            Self::DemasiadosFicheros => write!(f, "pasa de {MAX_FICHEROS_ADDON} ficheros"),
            Self::RutaNoValida(ruta) => write!(f, "ruta no válida: {ruta}"),
            Self::Enlace(ruta) => write!(f, "enlace: {ruta}"),
            Self::NoSeLee(detalle) => write!(f, "no se lee: {detalle}"),
        }
    }
}

/// Un fichero del addon: su ruta relativa (`/` como separador) y su contenido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FicheroAddon {
    pub ruta: String,
    pub contenido: Vec<u8>,
}

/// Empaqueta `carpeta` en un ZIP. Los enlaces y uniones no se siguen: hacen fallar, porque podrían sacar ficheros de
/// fuera del addon.
pub fn empaquetar(carpeta: &Path) -> Result<Vec<u8>, ErrorAddon> {
    let mut ficheros = Vec::new();
    recorrer(carpeta, "", 0, &mut ficheros)?;
    let opciones = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for fichero in &ficheros {
        zip.start_file(fichero.ruta.as_str(), opciones)
            .and_then(|()| zip.write_all(&fichero.contenido).map_err(Into::into))
            .map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?;
    }
    zip.finish()
        .map(Cursor::into_inner)
        .map_err(|e| ErrorAddon::NoSeLee(e.to_string()))
}

fn recorrer(
    carpeta: &Path,
    prefijo: &str,
    profundidad: usize,
    ficheros: &mut Vec<FicheroAddon>,
) -> Result<(), ErrorAddon> {
    if profundidad > MAX_PROFUNDIDAD {
        return Err(ErrorAddon::RutaNoValida(prefijo.to_owned()));
    }
    let mut entradas: Vec<_> = std::fs::read_dir(carpeta)
        .and_then(|d| d.collect::<Result<Vec<_>, _>>())
        .map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?;
    // Mismo orden en cualquier equipo: el paquete sale igual.
    entradas.sort_by_key(std::fs::DirEntry::file_name);
    for entrada in entradas {
        let nombre = entrada.file_name().to_string_lossy().into_owned();
        let ruta = format!("{prefijo}{nombre}");
        if !segmento_literal(&nombre) {
            return Err(ErrorAddon::RutaNoValida(ruta));
        }
        let tipo = entrada
            .file_type()
            .map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?;
        if tipo.is_symlink() {
            return Err(ErrorAddon::Enlace(ruta));
        }
        if tipo.is_dir() {
            recorrer(
                &entrada.path(),
                &format!("{ruta}/"),
                profundidad + 1,
                ficheros,
            )?;
        } else {
            anadir(ficheros, ruta, &entrada.path())?;
        }
    }
    Ok(())
}

fn anadir(ficheros: &mut Vec<FicheroAddon>, ruta: String, origen: &Path) -> Result<(), ErrorAddon> {
    if ficheros.len() >= MAX_FICHEROS_ADDON {
        return Err(ErrorAddon::DemasiadosFicheros);
    }
    let contenido = std::fs::read(origen).map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?;
    let total: u64 = ficheros.iter().map(|f| f.contenido.len() as u64).sum();
    if total + contenido.len() as u64 > MAX_BYTES_ADDON {
        return Err(ErrorAddon::DemasiadoGrande);
    }
    ficheros.push(FicheroAddon { ruta, contenido });
    Ok(())
}

/// Lee un ZIP de addon con todas las comprobaciones: cada nombre es una ruta relativa de segmentos válidos en Windows,
/// sin enlaces, sin repetidos (sin distinguir mayúsculas) y sin pasar de los límites al descomprimir.
pub fn leer(zip: &[u8]) -> Result<Vec<FicheroAddon>, ErrorAddon> {
    let mut archivo =
        ZipArchive::new(Cursor::new(zip)).map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?;
    if archivo.len() > MAX_FICHEROS_ADDON {
        return Err(ErrorAddon::DemasiadosFicheros);
    }
    let mut ficheros: Vec<FicheroAddon> = Vec::new();
    let mut vistas = std::collections::HashSet::new();
    let mut total: u64 = 0;
    for i in 0..archivo.len() {
        let mut entrada = archivo
            .by_index(i)
            .map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?;
        let ruta = entrada
            .name()
            .map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?
            .into_owned();
        if entrada.is_dir() {
            continue;
        }
        if entrada.is_symlink() {
            return Err(ErrorAddon::Enlace(ruta));
        }
        if !ruta_relativa_valida(&ruta) || !vistas.insert(ruta.to_lowercase()) {
            return Err(ErrorAddon::RutaNoValida(ruta));
        }
        let mut contenido = Vec::new();
        let queda = MAX_BYTES_ADDON - total;
        (&mut entrada)
            .take(queda + 1)
            .read_to_end(&mut contenido)
            .map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?;
        total += contenido.len() as u64;
        if total > MAX_BYTES_ADDON {
            return Err(ErrorAddon::DemasiadoGrande);
        }
        ficheros.push(FicheroAddon { ruta, contenido });
    }
    Ok(ficheros)
}

/// `a/b/c.lua`: segmentos literales de Windows separados por `/`, sin `..` ni `.`, sin raíz ni unidad.
fn ruta_relativa_valida(ruta: &str) -> bool {
    let segmentos: Vec<&str> = ruta.split('/').collect();
    segmentos.len() <= MAX_PROFUNDIDAD + 1
        && segmentos
            .iter()
            .all(|s| *s != "." && *s != ".." && segmento_literal(s))
}

#[cfg(test)]
mod tests;
