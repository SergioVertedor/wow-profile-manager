//! Errores que el núcleo devuelve a la interfaz. Siguen la convención de la #9: un código estable y sus datos,
//! nunca un texto para el usuario; la interfaz los traduce con `src/textos/`.

use serde::Serialize;
use std::path::Path;

/// Se serializa como `{ "codigo": "lecturaFallida", "datos": { … } }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "codigo", content = "datos", rename_all = "camelCase")]
pub enum ErrorNucleo {
    /// No se pudo leer un fichero o una carpeta. `detalle` es el mensaje del sistema operativo.
    LecturaFallida { ruta: String, detalle: String },
    /// El sistema no pudo abrir un enlace (por ejemplo, sin navegador por defecto).
    EnlaceNoAbierto { url: String, detalle: String },
    /// No se pudo escribir un fichero. `detalle` es el mensaje del sistema operativo.
    EscrituraFallida { ruta: String, detalle: String },
    /// Hay un `Wow.exe` en marcha: al salir, el juego reescribiría el `WTF`. No se escribe nada.
    JuegoAbierto {},
    /// El personaje pedido no está entre los que da Descubrimiento para esa instalación y cuenta.
    PersonajeNoEncontrado {},
    /// Hay un enlace o una unión de Windows en la carpeta que se iba a tocar: seguirlo podría salir del `WTF`.
    EnlaceNoAdmitido { ruta: String },
    /// Una operación falló. `restaurada`: el `WTF` quedó como antes. Si no, las copias siguen en la operación `id`.
    OperacionFallida {
        id: String,
        restaurada: bool,
        detalle: String,
    },
    /// No hay una operación con ese id en la carpeta de operaciones (o su diario no se entiende).
    OperacionNoEncontrada {},
    /// La operación no se puede deshacer: no es una eliminación ni una importación `completa`.
    NoDeshacible {},
    /// La operación no se restaura a petición: solo una importación cortada a mitad o una que no se pudo restaurar.
    NoRestaurable {},
    /// Una operación sin resolver (`id`: importación a medias o sin restaurar del todo): hasta restaurarla, o borrarla
    /// tras devolverla a mano, no se crea ninguna otra.
    OperacionAMedias { id: String },
    /// Una operación posterior `completa` (`id`) tocó alguno de los mismos ficheros: hay que deshacerla antes.
    OperacionPosterior { id: String },
    /// La raíz de la versión ya no existe o su producto no es el de la operación. No se escribe nada.
    JuegoDistinto { raiz: String },
    /// Algún fichero cambió desde que se enseñó qué iba a pasar. No se escribe nada: hay que volver a mirarlo.
    PlanCaducado {},
    /// La operación no ha terminado (está a medias): no se puede borrar.
    OperacionEnCurso {},
    /// La operación no se pudo restaurar del todo: solo se borra si el usuario confirma que ya restauró a mano.
    RestauracionManualSinConfirmar {},
    /// El sistema no pudo abrir una carpeta en el explorador.
    CarpetaNoAbierta { ruta: String, detalle: String },
    /// No se pudo consultar o descargar una actualización (red, GitHub). `detalle`, para el diagnóstico.
    ActualizacionNoDescargada { detalle: String },
    /// La actualización descargada no trae una firma válida de la app: no se instala (#3).
    ActualizacionSinFirmaValida {},
    /// Se pidió instalar y ya no hay una versión más nueva publicada.
    ActualizacionNoDisponible {},
    /// `preferencias.json` existe pero no se entiende. No se sobrescribe: arreglarlo o borrarlo es del usuario.
    PreferenciasIlegibles { ruta: String },
    /// No se sabe la versión del cliente o su número de interfaz (`.build.info`, `Config.wtf`): el paquete los exige.
    ExportacionSinVersion {},
    /// El nombre del perfil está vacío o pasa de 80 caracteres, o la descripción pasa de 500.
    PerfilNoValido {},
    /// No se ha marcado ninguna pieza que exista.
    SeleccionVacia {},
    /// Se pasa de los límites del formato (256 piezas, 32 MiB por pieza, 128 MiB en total; 64 MiB por addon y 512 MiB
    /// de addons).
    PaqueteDemasiadoGrande {},
    /// La carpeta de un addon no se pudo empaquetar al guardar (cambió desde la propuesta, un enlace, un nombre que
    /// Windows no admite…). `detalle`, para el diagnóstico.
    AddonNoEmpaquetado { addon: String, detalle: String },
    /// Un nombre propio demasiado largo o con caracteres de control.
    NombreNoValido { maximo: usize },
    /// El `.wowprofile` no cumple el formato. `detalle` dice dónde (para diagnóstico, no para el usuario).
    PaqueteInvalido {
        motivo: MotivoPaquete,
        detalle: String,
    },
    /// El paquete es de una versión mayor del formato que esta app no entiende: hay que actualizar la app.
    VersionPaquete { version: String },
    /// El paquete pedido no es el que está abierto (o no hay ninguno): hay que volver a abrirlo.
    PaqueteNoAbierto {},
    /// Se pidió aplicar con entradas de cuenta sin decidir: la interfaz no debería dejar.
    ConflictosSinDecidir {},
    /// Al aplicar, el `.wowprofile` ya no está donde se abrió o su contenido cambió: hay que volver a abrirlo.
    PaqueteCambiado {},
    /// Los personajes destino están vacíos, repetidos, son de varias instalaciones o de otra versión del juego.
    DestinosNoValidos {},
}

/// Qué paso de «Leer un paquete de forma segura» (`docs/wowprofile-format.md`) falló.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MotivoPaquete {
    /// No es un ZIP que se pueda abrir.
    NoEsZip,
    /// El fichero pasa del tamaño máximo de un paquete.
    DemasiadoGrande,
    /// Más de 257 entradas en el ZIP.
    DemasiadasEntradas,
    /// `manifest.json` falta, pasa de 1 MiB o no es JSON.
    ManifiestoIlegible,
    /// `manifest.json` no cumple el schema.
    ManifiestoNoValido,
    /// El ZIP no tiene exactamente `manifest.json` y un `datos/<id>` por entrada.
    EntradasDelZip,
    /// Una pieza no se puede descomprimir.
    PiezaIlegible,
    /// Una pieza descomprime más de lo que dice el manifiesto (ZIP bomba).
    PiezaDemasiadoGrande,
    /// Las piezas pasan de 128 MiB en total, o las de addons de 512 MiB.
    TotalDemasiadoGrande,
    /// El ZIP de un addon no es seguro: una ruta que sale de su carpeta, un nombre que Windows no admite, un enlace o
    /// demasiado contenido (#73).
    AddonNoValido,
    /// El tamaño o el hash de una pieza no cuadran.
    PiezaDanada,
}

impl ErrorNucleo {
    pub fn lectura(ruta: &Path, error: &std::io::Error) -> Self {
        Self::LecturaFallida {
            ruta: ruta.display().to_string(),
            detalle: error.to_string(),
        }
    }

    /// Una línea para un diario o para el campo `detalle` de otro error: la ruta y el mensaje del sistema si los hay.
    pub fn detalle(&self) -> String {
        match self {
            Self::LecturaFallida { ruta, detalle } | Self::EscrituraFallida { ruta, detalle } => {
                format!("{ruta}: {detalle}")
            }
            Self::EnlaceNoAdmitido { ruta } | Self::PreferenciasIlegibles { ruta } => ruta.clone(),
            otro => serde_json::to_value(otro)
                .ok()
                .and_then(|v| v["codigo"].as_str().map(str::to_owned))
                .unwrap_or_default(),
        }
    }

    pub fn escritura(ruta: &Path, error: &dyn std::fmt::Display) -> Self {
        Self::EscrituraFallida {
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

    /// La interfaz reconoce un error del núcleo porque trae `codigo` y `datos`: también los que no tienen datos.
    #[test]
    fn un_error_sin_datos_lleva_datos_vacios() {
        let json = serde_json::to_value(ErrorNucleo::JuegoAbierto {}).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "codigo": "juegoAbierto", "datos": {} })
        );
    }
}
