//! Lecturas de disco que comparten Detección y Descubrimiento. Solo lectura: este módulo no tiene nada que escriba.

use crate::error::ErrorNucleo;
use std::path::{Path, PathBuf};

/// Clave con la que se compara una ruta. Windows no distingue mayúsculas, acepta `/` como separador y un separador
/// final: `C:\WoW\`, `c:\wow` y `C:/WoW` son la misma carpeta.
pub fn clave_de_ruta(ruta: &Path) -> String {
    ruta.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

/// Una subcarpeta con su nombre ya convertido a texto.
pub struct Subcarpeta {
    pub nombre: String,
    pub ruta: PathBuf,
}

/// Subcarpetas directas de `carpeta`, ordenadas por nombre para que el resultado no dependa del sistema de ficheros.
/// Los ficheros se saltan. Si la carpeta no existe, no hay subcarpetas.
pub fn subcarpetas(carpeta: &Path) -> Result<Vec<Subcarpeta>, ErrorNucleo> {
    if !carpeta.is_dir() {
        return Ok(Vec::new());
    }
    let entradas = std::fs::read_dir(carpeta).map_err(|e| ErrorNucleo::lectura(carpeta, &e))?;
    let mut resultado = Vec::new();
    for entrada in entradas {
        let entrada = entrada.map_err(|e| ErrorNucleo::lectura(carpeta, &e))?;
        let ruta = entrada.path();
        if ruta.is_dir() {
            resultado.push(Subcarpeta {
                nombre: entrada.file_name().to_string_lossy().into_owned(),
                ruta,
            });
        }
    }
    resultado.sort_by(|a, b| a.nombre.cmp(&b.nombre));
    Ok(resultado)
}

/// Ficheros directos de `carpeta` con esa extensión (sin distinguir mayúsculas), ordenados por ruta. Si la carpeta
/// no existe, no hay ficheros.
pub fn ficheros_con_extension(
    carpeta: &Path,
    extension: &str,
) -> Result<Vec<PathBuf>, ErrorNucleo> {
    if !carpeta.is_dir() {
        return Ok(Vec::new());
    }
    let entradas = std::fs::read_dir(carpeta).map_err(|e| ErrorNucleo::lectura(carpeta, &e))?;
    let mut resultado = Vec::new();
    for entrada in entradas {
        let ruta = entrada
            .map_err(|e| ErrorNucleo::lectura(carpeta, &e))?
            .path();
        let coincide = ruta
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(extension));
        if coincide && ruta.is_file() {
            resultado.push(ruta);
        }
    }
    resultado.sort();
    Ok(resultado)
}

/// Lee un fichero de texto si existe. Los bytes que no son UTF-8 se sustituyen: estos ficheros son ASCII y basta
/// con no fallar si alguno no lo es.
pub fn leer_texto(ruta: &Path) -> Result<Option<String>, ErrorNucleo> {
    if !ruta.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(ruta).map_err(|e| ErrorNucleo::lectura(ruta, &e))?;
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}

/// Un enlace simbólico o una unión de Windows (las dos son «sustitutos de nombre» para `is_symlink`).
pub fn es_enlace(ruta: &Path) -> bool {
    std::fs::symlink_metadata(ruta).is_ok_and(|m| m.file_type().is_symlink())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_clave_de_ruta_iguala_mayusculas_y_separadores() {
        let clave = clave_de_ruta(Path::new(r"C:\Juegos\World of Warcraft"));
        for otra in [
            r"c:\juegos\world of warcraft\",
            "C:/Juegos/World of Warcraft",
            "c:/juegos/world of warcraft/",
        ] {
            assert_eq!(clave_de_ruta(Path::new(otra)), clave, "{otra}");
        }
    }

    #[test]
    fn lista_solo_carpetas_y_en_orden() {
        let tmp = tempfile::tempdir().unwrap();
        for nombre in ["b", "a", "c"] {
            std::fs::create_dir(tmp.path().join(nombre)).unwrap();
        }
        std::fs::write(tmp.path().join("fichero.txt"), "x").unwrap();

        let nombres: Vec<_> = subcarpetas(tmp.path())
            .unwrap()
            .into_iter()
            .map(|s| s.nombre)
            .collect();

        assert_eq!(nombres, ["a", "b", "c"]);
    }

    #[test]
    fn una_carpeta_que_no_existe_no_tiene_subcarpetas() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(subcarpetas(&tmp.path().join("no")).unwrap().is_empty());
    }

    #[test]
    fn un_fichero_que_no_existe_es_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(leer_texto(&tmp.path().join("no")).unwrap(), None);
    }
}
