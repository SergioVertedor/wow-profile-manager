//! Carpeta de datos portable (#3, R-02): todo lo que la app guarda vive en una carpeta junto al `.exe`, sin
//! `%APPDATA%` ni registro de Windows. Se mueve el `.exe` con su carpeta y la app se viene entera. La regla es pura y
//! se prueba con carpetas temporales; solo `run` pregunta al sistema dónde está el `.exe`.

use crate::idioma::Idioma;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Nombre de la carpeta de datos, junto al `.exe`.
pub const CARPETA_DATOS: &str = "wow-profile-manager-datos";
/// Datos del propio WebView2 (caché, almacenamiento) dentro de la carpeta de datos.
pub const CARPETA_WEBVIEW: &str = "webview";
/// Fichero que se escribe y se borra para comprobar que se puede escribir.
const PRUEBA_ESCRITURA: &str = ".prueba-escritura";

/// Carpeta de datos que corresponde a un `.exe`: la de al lado.
pub fn carpeta_junto_a(exe: &Path) -> PathBuf {
    exe.parent().unwrap_or(Path::new("")).join(CARPETA_DATOS)
}

/// Crea la carpeta si no existe y comprueba que se puede escribir en ella. En Windows, el atributo de solo lectura no
/// refleja las ACL (`Program Files`), así que la única prueba fiable es escribir de verdad.
pub fn preparar(carpeta: &Path) -> io::Result<()> {
    fs::create_dir_all(carpeta)?;
    let prueba = carpeta.join(PRUEBA_ESCRITURA);
    fs::write(&prueba, b"")?;
    fs::remove_file(&prueba)
}

/// Mensaje para cuando la carpeta de datos no se puede usar: sin ella la app no arranca. Va en el idioma de Windows
/// porque todavía no hay interfaz que lo traduzca.
pub fn mensaje_sin_carpeta(idioma: Idioma, carpeta: &Path, error: &io::Error) -> String {
    let carpeta = carpeta.display();
    match idioma {
        Idioma::Es => format!(
            "No se puede escribir en la carpeta de datos de la app:\n{carpeta}\n\n{error}\n\n\
             La app guarda todo junto al ejecutable. Muévelo a una carpeta donde puedas escribir (por ejemplo, \
             Documentos o el escritorio) y ábrelo de nuevo."
        ),
        Idioma::En => format!(
            "The app cannot write to its data folder:\n{carpeta}\n\n{error}\n\n\
             The app keeps everything next to its executable. Move it to a folder you can write to (for example, \
             Documents or the desktop) and open it again."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_carpeta_de_datos_esta_junto_al_exe() {
        let exe = Path::new(r"D:\juegos\wpm\wow-profile-manager.exe");
        assert_eq!(
            carpeta_junto_a(exe),
            Path::new(r"D:\juegos\wpm").join("wow-profile-manager-datos")
        );
    }

    #[test]
    fn preparar_crea_la_carpeta_y_no_deja_rastro() {
        let tmp = tempfile::tempdir().unwrap();
        let carpeta = carpeta_junto_a(&tmp.path().join("app.exe"));

        preparar(&carpeta).unwrap();

        assert!(carpeta.is_dir());
        assert_eq!(fs::read_dir(&carpeta).unwrap().count(), 0);
    }

    #[test]
    fn preparar_respeta_lo_que_ya_hay() {
        let tmp = tempfile::tempdir().unwrap();
        let carpeta = carpeta_junto_a(&tmp.path().join("app.exe"));
        fs::create_dir_all(&carpeta).unwrap();
        fs::write(carpeta.join("preferencias.json"), "{}").unwrap();

        preparar(&carpeta).unwrap();

        assert_eq!(
            fs::read_to_string(carpeta.join("preferencias.json")).unwrap(),
            "{}"
        );
    }

    #[test]
    fn si_la_carpeta_no_se_puede_crear_es_un_error() {
        let tmp = tempfile::tempdir().unwrap();
        // Un fichero donde tendría que ir una carpeta: `create_dir_all` no puede crearla.
        let ocupado = tmp.path().join(CARPETA_DATOS);
        fs::write(&ocupado, "no soy una carpeta").unwrap();

        assert!(preparar(&ocupado).is_err());
    }

    #[test]
    fn el_mensaje_dice_la_carpeta_en_el_idioma_de_windows() {
        let carpeta = Path::new(r"C:\Program Files\wpm\wow-profile-manager-datos");
        let error = io::Error::from(io::ErrorKind::PermissionDenied);

        let es = mensaje_sin_carpeta(Idioma::Es, carpeta, &error);
        let en = mensaje_sin_carpeta(Idioma::En, carpeta, &error);

        assert!(es.contains(r"C:\Program Files\wpm\wow-profile-manager-datos"));
        assert!(es.starts_with("No se puede escribir"));
        assert!(en.contains(r"C:\Program Files\wpm\wow-profile-manager-datos"));
        assert!(en.starts_with("The app cannot write"));
    }
}
