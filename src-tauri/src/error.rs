//! Errores que el núcleo devuelve a la interfaz. Siguen la convención de la spec 011: un código estable y sus datos,
//! nunca un texto para el usuario; la interfaz los traduce con `src/textos/`.

use serde::Serialize;
use std::path::Path;

/// Se serializa como `{ "codigo": "lecturaFallida", "datos": { … } }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "codigo", content = "datos", rename_all = "camelCase")]
pub enum ErrorNucleo {
    /// No se pudo leer un fichero o una carpeta. `detalle` es el mensaje del sistema operativo.
    LecturaFallida { ruta: String, detalle: String },
}

impl ErrorNucleo {
    pub fn lectura(ruta: &Path, error: &std::io::Error) -> Self {
        Self::LecturaFallida {
            ruta: ruta.display().to_string(),
            detalle: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_serializa_como_codigo_y_datos() {
        let error = ErrorNucleo::lectura(Path::new("C:/x"), &std::io::Error::other("sin permiso"));
        let json = serde_json::to_value(error).unwrap();
        assert_eq!(json["codigo"], "lecturaFallida");
        assert_eq!(json["datos"]["ruta"], "C:/x");
        assert_eq!(json["datos"]["detalle"], "sin permiso");
    }
}
