//! Lo que toca la red y el disco al actualizar. Solo se llama con el interruptor encendido y en una versión publicada.

use super::{CLAVE_PUBLICA, Disponible, Release, URL_ULTIMA, Version, disponible, verificar};
use crate::error::ErrorNucleo;
use std::path::Path;
use std::time::Duration;
use ureq::Agent;
use ureq::tls::{TlsConfig, TlsProvider};

/// Ni el `.exe` ni el JSON de la release pasan de aquí: lo que sea más grande no es una release de esta app.
const MAX_DESCARGA: u64 = 64 * 1024 * 1024;
const PLAZO: Duration = Duration::from_secs(60);
/// Nombre genérico: no se manda nada que identifique a quien usa la app.
const AGENTE: &str = "wow-profile-manager";
const FICHERO_NUEVO: &str = "wow-profile-manager-nuevo.exe";

/// HTTPS con el TLS y los certificados de Windows.
fn agente() -> Agent {
    Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .build(),
        )
        .timeout_global(Some(PLAZO))
        .https_only(true)
        .user_agent(AGENTE)
        .build()
        .new_agent()
}

fn sin_descarga(error: impl std::fmt::Display) -> ErrorNucleo {
    ErrorNucleo::ActualizacionNoDescargada {
        detalle: error.to_string(),
    }
}

fn descargar(agente: &Agent, url: &str) -> Result<Vec<u8>, ErrorNucleo> {
    agente
        .get(url)
        .call()
        .map_err(sin_descarga)?
        .body_mut()
        .with_config()
        .limit(MAX_DESCARGA)
        .read_to_vec()
        .map_err(sin_descarga)
}

/// La versión publicada más nueva que `actual`, si la hay.
pub fn buscar(actual: Version) -> Result<Option<Disponible>, ErrorNucleo> {
    let agente = agente();
    let bytes = descargar(&agente, URL_ULTIMA)?;
    let release: Release = serde_json::from_slice(&bytes).map_err(sin_descarga)?;
    Ok(disponible(&release, actual))
}

/// Descarga la versión, comprueba su firma y, solo si es buena, sustituye el `.exe` en uso. Lo descargado se escribe
/// en `carpeta` (la de datos de la app) y se borra al terminar. Si algo falla, el `.exe` de ahora sigue como estaba.
pub fn instalar(disponible: &Disponible, carpeta: &Path) -> Result<(), ErrorNucleo> {
    let agente = agente();
    let exe = descargar(&agente, &disponible.url_exe)?;
    let firma = descargar(&agente, &disponible.url_firma)?;
    verificar(&exe, &firma, CLAVE_PUBLICA).map_err(|motivo| {
        log::error!(
            "actualización {} con firma no válida: {motivo:?}",
            disponible.version
        );
        ErrorNucleo::ActualizacionSinFirmaValida {}
    })?;
    let nuevo = carpeta.join(FICHERO_NUEVO);
    std::fs::write(&nuevo, &exe).map_err(|e| ErrorNucleo::escritura(&nuevo, &e))?;
    let sustituido =
        self_replace::self_replace(&nuevo).map_err(|e| ErrorNucleo::escritura(&nuevo, &e));
    if let Err(error) = std::fs::remove_file(&nuevo) {
        log::warn!("no se pudo borrar {}: {error}", nuevo.display());
    }
    sustituido
}
