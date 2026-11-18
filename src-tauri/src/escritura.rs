//! Escritura segura (`docs/flujos.md`): todo fichero que escribe la app se escribe a un temporal en la misma carpeta,
//! se fuerza a disco y se renombra encima del destino. Un corte, incluso de corriente, deja el fichero anterior o el
//! nuevo, nunca uno a medias.

use crate::error::ErrorNucleo;
use std::io::Write;
use std::path::Path;

/// Sufijo del temporal. Va en la misma carpeta que el destino: un renombrado entre unidades no sería atómico.
const SUFIJO_TEMPORAL: &str = ".tmp";

/// Escribe `bytes` en `ruta` de forma atómica y crea las carpetas que falten.
pub fn escribir_atomico(ruta: &Path, bytes: &[u8]) -> Result<(), ErrorNucleo> {
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
    let mut salida =
        std::fs::File::create(&temporal).map_err(|e| ErrorNucleo::escritura(&temporal, &e))?;
    salida
        .write_all(bytes)
        .and_then(|()| salida.sync_all())
        .map_err(|e| ErrorNucleo::escritura(&temporal, &e))?;
    drop(salida);
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
}
