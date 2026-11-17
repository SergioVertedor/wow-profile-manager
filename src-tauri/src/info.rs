//! Datos de la propia aplicación. Lógica pura: no depende de Tauri y se prueba sin abrir ninguna ventana.

use crate::enlaces::{URL_KO_FI, URL_LICENCIA, URL_REPOSITORIO};
use serde::Serialize;

/// Nombre visible de la aplicación.
pub const NOMBRE_APP: &str = "WoW Profile Manager";

/// Lo que la interfaz necesita saber de la aplicación.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoApp {
    pub nombre: &'static str,
    pub version: &'static str,
    pub autor: &'static str,
    /// URL que la pantalla «Acerca de» enseña. Para abrirlas, la interfaz pide el `Enlace` por su nombre.
    pub repositorio: &'static str,
    pub ko_fi: &'static str,
    pub licencia: &'static str,
}

/// Información de la aplicación. Versión, autor y repositorio salen de `Cargo.toml` al compilar, así que no se pueden
/// desincronizar.
pub fn info_app() -> InfoApp {
    InfoApp {
        nombre: NOMBRE_APP,
        version: env!("CARGO_PKG_VERSION"),
        autor: env!("CARGO_PKG_AUTHORS"),
        repositorio: URL_REPOSITORIO,
        ko_fi: URL_KO_FI,
        licencia: URL_LICENCIA,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_version_es_la_de_cargo_toml() {
        assert_eq!(info_app().version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn el_nombre_es_el_visible() {
        assert_eq!(info_app().nombre, "WoW Profile Manager");
    }

    #[test]
    fn se_serializa_con_los_campos_que_lee_la_interfaz() {
        let json = serde_json::to_value(info_app()).unwrap();
        assert_eq!(json["nombre"], "WoW Profile Manager");
        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(json["autor"], "Sergio Vertedor");
        assert_eq!(json["koFi"], "https://ko-fi.com/tonotabidev");
        assert_eq!(
            json["repositorio"],
            "https://github.com/SergioVertedor/wow-profile-manager"
        );
    }
}
