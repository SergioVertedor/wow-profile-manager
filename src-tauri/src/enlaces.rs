//! Enlaces que la app abre en el navegador del sistema. La lista es cerrada y vive aquí: la interfaz pide un
//! `Enlace` por su nombre y nunca pasa una URL, así que no puede abrir una dirección arbitraria aunque alguien
//! inyecte código en la ventana.

use crate::error::ErrorNucleo;
use serde::{Deserialize, Serialize};

/// Repositorio del proyecto. Sale de `Cargo.toml`, igual que la versión.
pub const URL_REPOSITORIO: &str = env!("CARGO_PKG_REPOSITORY");
/// Ko-fi del proyecto (`.github/FUNDING.yml`: `ko_fi: tonotabidev`).
pub const URL_KO_FI: &str = "https://ko-fi.com/tonotabidev";
/// Web del autor.
pub const URL_WEB: &str = "https://tonotabi.dev";
/// Texto oficial de la GPL-3.0. `LICENSE` no está en `main` hasta la primera release.
pub const URL_LICENCIA: &str = "https://www.gnu.org/licenses/gpl-3.0.html";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Enlace {
    Repositorio,
    KoFi,
    Web,
    Licencia,
}

impl Enlace {
    pub const fn url(self) -> &'static str {
        match self {
            Self::Repositorio => URL_REPOSITORIO,
            Self::KoFi => URL_KO_FI,
            Self::Web => URL_WEB,
            Self::Licencia => URL_LICENCIA,
        }
    }
}

/// Abre el enlace con el navegador por defecto, sin esperar a que se cierre.
pub fn abrir(enlace: Enlace) -> Result<(), ErrorNucleo> {
    open::that_detached(enlace.url()).map_err(|e| ErrorNucleo::EnlaceNoAbierto {
        url: enlace.url().to_owned(),
        detalle: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_enlace_va_a_su_direccion_https() {
        assert_eq!(
            Enlace::Repositorio.url(),
            "https://github.com/SergioVertedor/wow-profile-manager"
        );
        assert_eq!(Enlace::KoFi.url(), "https://ko-fi.com/tonotabidev");
        assert_eq!(Enlace::Web.url(), "https://tonotabi.dev");
        assert_eq!(
            Enlace::Licencia.url(),
            "https://www.gnu.org/licenses/gpl-3.0.html"
        );
    }

    #[test]
    fn la_interfaz_los_pide_por_estos_nombres() {
        for (enlace, nombre) in [
            (Enlace::Repositorio, "repositorio"),
            (Enlace::KoFi, "koFi"),
            (Enlace::Web, "web"),
            (Enlace::Licencia, "licencia"),
        ] {
            assert_eq!(serde_json::to_value(enlace).unwrap(), nombre);
            assert_eq!(
                serde_json::from_value::<Enlace>(nombre.into()).unwrap(),
                enlace
            );
        }
    }

    #[test]
    fn una_url_o_un_nombre_desconocido_no_es_un_enlace() {
        for valor in ["https://example.com", "otro", ""] {
            assert!(
                serde_json::from_value::<Enlace>(valor.into()).is_err(),
                "{valor}"
            );
        }
    }
}
