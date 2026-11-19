//! Escritura segura (`docs/flujos.md`): todo fichero que escribe la app se escribe a un temporal en la misma carpeta,
//! se fuerza a disco y se renombra encima del destino. Un corte, incluso de corriente, deja el fichero anterior o el
//! nuevo, nunca uno a medias.

use crate::error::ErrorNucleo;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Sufijo del temporal. Va en la misma carpeta que el destino: un renombrado entre unidades no sería atómico.
const SUFIJO_TEMPORAL: &str = ".tmp";

/// Escribe `bytes` en `ruta` de forma atómica y crea las carpetas que falten.
pub fn escribir_atomico(ruta: &Path, bytes: &[u8]) -> Result<(), ErrorNucleo> {
    escribir_atomico_con(ruta, |salida| {
        salida
            .write_all(bytes)
            .map_err(|e| ErrorNucleo::escritura(ruta, &e))
    })
}

/// Como `escribir_atomico`, pero el contenido lo va escribiendo `escribir` en el temporal: sirve para ficheros que no
/// conviene tener enteros en memoria. Si `escribir` falla, el temporal se borra y el destino no cambia.
pub fn escribir_atomico_con(
    ruta: &Path,
    escribir: impl FnOnce(&mut File) -> Result<(), ErrorNucleo>,
) -> Result<(), ErrorNucleo> {
    let Some(carpeta) = ruta.parent() else {
        return Err(ErrorNucleo::escritura(
            ruta,
            &std::io::Error::other("sin carpeta"),
        ));
    };
    std::fs::create_dir_all(carpeta).map_err(|e| ErrorNucleo::escritura(carpeta, &e))?;
    let mut nombre = ruta.file_name().unwrap_or_default().to_os_string();
    nombre.push(SUFIJO_TEMPORAL);
    let temporal = carpeta.join(nombre);
    let mut salida = File::create(&temporal).map_err(|e| ErrorNucleo::escritura(&temporal, &e))?;
    let escrito = escribir(&mut salida).and_then(|()| {
        salida
            .sync_all()
            .map_err(|e| ErrorNucleo::escritura(&temporal, &e))
    });
    drop(salida);
    if let Err(error) = escrito {
        // Lo que importa es el error de escribir; si el temporal no se puede borrar, se queda.
        let _ = std::fs::remove_file(&temporal);
        return Err(error);
    }
    std::fs::rename(&temporal, ruta).map_err(|e| ErrorNucleo::escritura(ruta, &e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escribe_crea_carpetas_y_no_deja_temporal() {
        let tmp = tempfile::tempdir().unwrap();
        let ruta = tmp.path().join("a").join("b").join("f.json");
        escribir_atomico(&ruta, b"uno").unwrap();
        escribir_atomico(&ruta, b"dos").unwrap();
        assert_eq!(std::fs::read(&ruta).unwrap(), b"dos");
        assert!(!tmp.path().join("a").join("b").join("f.json.tmp").exists());
    }

    #[test]
    fn si_escribir_falla_el_destino_no_cambia_y_no_queda_temporal() {
        let tmp = tempfile::tempdir().unwrap();
        let ruta = tmp.path().join("f.json");
        escribir_atomico(&ruta, b"antes").unwrap();
        let fallo = escribir_atomico_con(&ruta, |salida| {
            salida.write_all(b"a medias").unwrap();
            Err(ErrorNucleo::PaqueteDemasiadoGrande {})
        });
        assert_eq!(fallo, Err(ErrorNucleo::PaqueteDemasiadoGrande {}));
        assert_eq!(std::fs::read(&ruta).unwrap(), b"antes");
        assert!(!tmp.path().join("f.json.tmp").exists());
    }
}
