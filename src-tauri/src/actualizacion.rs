//! Autoactualización (#3). La lógica es pura y se prueba sin red: comparar versiones, leer la última
//! release de GitHub y comprobar la firma. La red está en `red.rs`; sustituir el `.exe` y reiniciar, en `lib.rs`.
//!
//! Solo se instala un `.exe` cuya firma minisign (la de `tauri signer`) cuadra con [`CLAVE_PUBLICA`]. La privada no
//! está en el repo: firma la CI con el secreto `TAURI_SIGNING_PRIVATE_KEY`.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use std::fmt;

mod red;
pub use red::{buscar, instalar};

/// Clave pública de las actualizaciones (`minisign public key: 09995B8D36B4DB80`).
pub const CLAVE_PUBLICA: &str = "RWSA27Q2jVuZCcGyovPjw7BnNl+p1E4/2GQF7YEJUdM7soLQvch+YtLW";
/// La última release del repositorio, en la API de GitHub.
pub const URL_ULTIMA: &str =
    "https://api.github.com/repos/SergioVertedor/wow-profile-manager/releases/latest";
/// Los ficheros de cada release (los sube `.github/workflows/release.yml`).
pub const NOMBRE_EXE: &str = "wow-profile-manager.exe";
pub const NOMBRE_FIRMA: &str = "wow-profile-manager.exe.sig";

/// Una versión `X.Y.Z`. Se compara por números, no como texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u32, u32, u32);

impl Version {
    /// Por si la versión propia no se pudiera leer: cualquier publicada sería más nueva, pero solo se instala firmada.
    pub const MINIMA: Self = Self(0, 0, 0);

    /// `1.2.3` o `v1.2.3`. Cualquier otra cosa (sufijos incluidos) no es una versión que se instale.
    pub fn leer(texto: &str) -> Option<Self> {
        let texto = texto.strip_prefix('v').unwrap_or(texto);
        let mut partes = texto.split('.').map(|p| p.parse::<u32>().ok());
        let version = Self(partes.next()??, partes.next()??, partes.next()??);
        partes.next().is_none().then_some(version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Lo que importa del JSON de `releases/latest`.
#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub assets: Vec<Fichero>,
}

#[derive(Debug, Deserialize)]
pub struct Fichero {
    pub name: String,
    pub browser_download_url: String,
}

/// Una versión más nueva que la instalada, con sus dos ficheros.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Disponible {
    pub version: String,
    /// Las notas de la release, tal cual.
    pub notas: String,
    #[serde(skip)]
    pub url_exe: String,
    #[serde(skip)]
    pub url_firma: String,
}

/// La release, si es más nueva que `actual` y trae el `.exe` y su firma. Una release rara no se ofrece.
pub fn disponible(release: &Release, actual: Version) -> Option<Disponible> {
    let version = Version::leer(&release.tag_name).filter(|v| *v > actual)?;
    let url = |nombre: &str| {
        release
            .assets
            .iter()
            .find(|f| f.name == nombre)
            .map(|f| f.browser_download_url.clone())
    };
    Some(Disponible {
        version: version.to_string(),
        notas: release.body.clone().unwrap_or_default(),
        url_exe: url(NOMBRE_EXE)?,
        url_firma: url(NOMBRE_FIRMA)?,
    })
}

/// Por qué una descarga no se instala.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirmaNoValida {
    /// El `.sig` no es base64 de una firma minisign.
    Formato,
    /// La firma no es de la clave de la app o el `.exe` no es el firmado.
    NoCuadra,
}

/// Comprueba que `exe` es el que firmó la clave `clave` (`.sig` de `tauri signer`: la firma minisign en base64).
pub fn verificar(exe: &[u8], sig: &[u8], clave: &str) -> Result<(), FirmaNoValida> {
    let clave = PublicKey::from_base64(clave).map_err(|_| FirmaNoValida::Formato)?;
    let texto = STANDARD
        .decode(sig.trim_ascii())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or(FirmaNoValida::Formato)?;
    let firma = Signature::decode(&texto).map_err(|_| FirmaNoValida::Formato)?;
    clave
        .verify(exe, &firma, false)
        .map_err(|_| FirmaNoValida::NoCuadra)
}

#[cfg(test)]
mod tests;
