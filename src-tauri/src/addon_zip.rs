//! La carpeta de un addon como pieza del paquete (#73): un ZIP con sus ficheros, con rutas relativas a la
//! carpeta. Empaquetar no sigue enlaces; leer comprueba cada nombre y los límites antes de devolver nada, porque lo que
//! sale de aquí se escribe en `Interface/AddOns` del que importa.

use crate::operaciones::segmento_literal;
use serde::Serialize;
use std::fmt;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Un addon más grande no viaja como código: solo con su enlace. Medido: 61 de 63 carpetas reales caben.
pub const MAX_BYTES_ADDON: u64 = 64 * 1024 * 1024;
/// Medido: la carpeta real con más ficheros tiene 3.083.
pub const MAX_FICHEROS_ADDON: usize = 4096;
/// Profundidad de carpetas dentro del addon. Ninguno real se acerca; corta recorridos absurdos.
const MAX_PROFUNDIDAD: usize = 32;

/// Por qué una carpeta no se empaqueta o un ZIP de addon no se acepta. Va a la interfaz como `{ codigo, detalle }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "codigo", content = "detalle", rename_all = "camelCase")]
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

/// Lo que ocupa la carpeta de un addon, sin comprimir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Medida {
    pub bytes: u64,
    pub ficheros: usize,
}

/// Un fichero encontrado al recorrer la carpeta: su ruta en el ZIP y en disco.
struct Encontrado {
    ruta: String,
    origen: PathBuf,
}

/// Recorre `carpeta` con las mismas comprobaciones que `empaquetar`, sin leer los ficheros: sirve para ofrecer el
/// addon (o decir por qué no viaja) sin comprimir nada.
pub fn medir(carpeta: &Path) -> Result<Medida, ErrorAddon> {
    let mut recorrido = Recorrido::default();
    recorrido.recorrer(carpeta, "", 0)?;
    Ok(Medida {
        bytes: recorrido.bytes,
        ficheros: recorrido.ficheros.len(),
    })
}

/// Empaqueta `carpeta` en un ZIP. Los enlaces y uniones no se siguen: hacen fallar, porque podrían sacar ficheros de
/// fuera del addon. El ZIP tampoco puede pasar de `MAX_BYTES_ADDON`: es lo que el formato admite por pieza.
pub fn empaquetar(carpeta: &Path) -> Result<Vec<u8>, ErrorAddon> {
    let mut recorrido = Recorrido::default();
    recorrido.recorrer(carpeta, "", 0)?;
    let opciones = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let mut total: u64 = 0;
    for fichero in &recorrido.ficheros {
        // El fichero puede haber crecido desde que se midió: no se lee más de lo que cabe.
        let mut contenido = Vec::new();
        std::fs::File::open(&fichero.origen)
            .and_then(|f| {
                f.take(MAX_BYTES_ADDON - total + 1)
                    .read_to_end(&mut contenido)
            })
            .map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?;
        total += contenido.len() as u64;
        if total > MAX_BYTES_ADDON {
            return Err(ErrorAddon::DemasiadoGrande);
        }
        zip.start_file(fichero.ruta.as_str(), opciones)
            .and_then(|()| zip.write_all(&contenido).map_err(Into::into))
            .map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?;
    }
    let bytes = zip
        .finish()
        .map(Cursor::into_inner)
        .map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?;
    if bytes.len() as u64 > MAX_BYTES_ADDON {
        return Err(ErrorAddon::DemasiadoGrande);
    }
    Ok(bytes)
}

/// Lo encontrado al recorrer una carpeta, con los límites ya comprobados.
#[derive(Default)]
struct Recorrido {
    ficheros: Vec<Encontrado>,
    bytes: u64,
}

impl Recorrido {
    fn recorrer(
        &mut self,
        carpeta: &Path,
        prefijo: &str,
        profundidad: usize,
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
            let nombre = entrada.file_name().into_string().map_err(|nombre| {
                ErrorAddon::RutaNoValida(format!("{prefijo}{}", nombre.to_string_lossy()))
            })?;
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
                self.recorrer(&entrada.path(), &format!("{ruta}/"), profundidad + 1)?;
            } else {
                let bytes = entrada
                    .metadata()
                    .map_err(|e| ErrorAddon::NoSeLee(e.to_string()))?
                    .len();
                self.anadir(ruta, entrada.path(), bytes)?;
            }
        }
        Ok(())
    }

    fn anadir(&mut self, ruta: String, origen: PathBuf, bytes: u64) -> Result<(), ErrorAddon> {
        if self.ficheros.len() >= MAX_FICHEROS_ADDON {
            return Err(ErrorAddon::DemasiadosFicheros);
        }
        self.bytes += bytes;
        if self.bytes > MAX_BYTES_ADDON {
            return Err(ErrorAddon::DemasiadoGrande);
        }
        self.ficheros.push(Encontrado { ruta, origen });
        Ok(())
    }
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
        if entrada.is_symlink() {
            return Err(ErrorAddon::Enlace(ruta));
        }
        if entrada.is_dir() {
            // Una carpeta no se escribe (salen de las rutas de sus ficheros), pero su nombre tampoco puede salirse.
            let carpeta = ruta.strip_suffix('/').unwrap_or(&ruta);
            if !ruta_relativa_valida(carpeta) {
                return Err(ErrorAddon::RutaNoValida(ruta));
            }
            continue;
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
pub(crate) fn ruta_relativa_valida(ruta: &str) -> bool {
    let segmentos: Vec<&str> = ruta.split('/').collect();
    segmentos.len() <= MAX_PROFUNDIDAD + 1
        && segmentos
            .iter()
            .all(|s| *s != "." && *s != ".." && segmento_literal(s))
}

#[cfg(test)]
mod tests;
