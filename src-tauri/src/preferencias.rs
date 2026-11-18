//! Preferencias (spec 018): lo que el usuario le dice a la app y tiene que recordarse entre arranques: las carpetas
//! de WoW añadidas a mano y los nombres propios de instalaciones y cuentas. Es el único módulo que escribe fuera de
//! Operaciones, y solo en la carpeta de datos de la app (`%APPDATA%\<identificador>`), nunca dentro de una
//! instalación.

use crate::disco::clave_de_ruta;
use crate::error::ErrorNucleo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const FICHERO: &str = "preferencias.json";
const TEMPORAL: &str = "preferencias.json.tmp";
/// Versión del formato del fichero. Uno de una versión posterior no se entiende y se trata como ilegible: así una
/// versión vieja de la app no lo pisa.
const VERSION: u32 = 1;
/// Cabe en el selector de versión y en la cabecera de la lista sin cortarse.
pub const LARGO_MAXIMO_NOMBRE: usize = 40;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferencias {
    /// Raíces elegidas a mano, tal como las dio el selector y en orden de alta.
    pub carpetas_manuales: Vec<PathBuf>,
    /// Clave: raíz normalizada (`clave_de_ruta`).
    pub nombres_instalacion: BTreeMap<String, String>,
    /// Clave: `clave_cuenta`.
    pub nombres_cuenta: BTreeMap<String, String>,
}

/// Lo que se guarda en disco: las preferencias con la versión del formato.
#[derive(Serialize, Deserialize)]
struct Fichero {
    version: u32,
    #[serde(flatten)]
    preferencias: Preferencias,
}

/// Separador de las claves de cuenta. Windows no lo admite en rutas ni en nombres de carpeta, así que no puede
/// aparecer dentro de ninguna parte.
const SEPARADOR: char = '|';

/// Una cuenta es única por instalación, versión del juego y carpeta de cuenta.
fn clave_cuenta(raiz: &Path, version: &str, cuenta: &str) -> String {
    format!(
        "{}{SEPARADOR}{version}{SEPARADOR}{cuenta}",
        clave_de_ruta(raiz)
    )
}

impl Preferencias {
    pub fn nombre_instalacion(&self, raiz: &Path) -> Option<String> {
        self.nombres_instalacion.get(&clave_de_ruta(raiz)).cloned()
    }

    pub fn nombre_cuenta(&self, raiz: &Path, version: &str, cuenta: &str) -> Option<String> {
        self.nombres_cuenta
            .get(&clave_cuenta(raiz, version, cuenta))
            .cloned()
    }

    pub fn es_manual(&self, raiz: &Path) -> bool {
        let clave = clave_de_ruta(raiz);
        self.carpetas_manuales
            .iter()
            .any(|carpeta| clave_de_ruta(carpeta) == clave)
    }

    /// Añade la raíz si no estaba ya (escrita de cualquier forma). Devuelve si la ha añadido.
    pub fn anadir_carpeta(&mut self, raiz: PathBuf) -> bool {
        if self.es_manual(&raiz) {
            return false;
        }
        self.carpetas_manuales.push(raiz);
        true
    }

    /// Quita una carpeta añadida a mano y, con ella, su nombre y los de sus cuentas.
    pub fn quitar_carpeta(&mut self, raiz: &Path) {
        let clave = clave_de_ruta(raiz);
        self.carpetas_manuales
            .retain(|carpeta| clave_de_ruta(carpeta) != clave);
        self.nombres_instalacion.remove(&clave);
        let prefijo = format!("{clave}{SEPARADOR}");
        self.nombres_cuenta
            .retain(|cuenta, _| !cuenta.starts_with(&prefijo));
    }

    /// `None` borra el nombre.
    pub fn nombrar_instalacion(&mut self, raiz: &Path, nombre: Option<String>) {
        poner(&mut self.nombres_instalacion, clave_de_ruta(raiz), nombre);
    }

    /// `None` borra el nombre.
    pub fn nombrar_cuenta(
        &mut self,
        raiz: &Path,
        version: &str,
        cuenta: &str,
        nombre: Option<String>,
    ) {
        poner(
            &mut self.nombres_cuenta,
            clave_cuenta(raiz, version, cuenta),
            nombre,
        );
    }
}

fn poner(nombres: &mut BTreeMap<String, String>, clave: String, nombre: Option<String>) {
    match nombre {
        Some(nombre) => nombres.insert(clave, nombre),
        None => nombres.remove(&clave),
    };
}

/// Comprueba lo que escribe el usuario. Se recorta; vacío es «sin nombre» (`None`). Los caracteres de control (saltos
/// de línea, tabuladores) se rechazan porque romperían una etiqueta de una línea.
pub fn nombre_valido(texto: &str) -> Result<Option<String>, ErrorNucleo> {
    // Igual que `String.prototype.trim` de la interfaz, que también quita el BOM (U+FEFF): si no, un nombre hecho solo
    // de BOM sería «sin nombre» en la ventana y un nombre invisible en el fichero.
    let nombre = texto.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if nombre.is_empty() {
        return Ok(None);
    }
    if nombre.chars().count() > LARGO_MAXIMO_NOMBRE || nombre.chars().any(char::is_control) {
        return Err(ErrorNucleo::NombreNoValido {
            maximo: LARGO_MAXIMO_NOMBRE,
        });
    }
    Ok(Some(nombre.to_owned()))
}

/// Lee las preferencias de `dir`. Sin fichero, vacías. Un fichero que no se entiende es un error: quien llama decide
/// seguir sin preferencias, pero nadie lo sobrescribe (D-04: nada se borra solo).
pub fn cargar(dir: &Path) -> Result<Preferencias, ErrorNucleo> {
    let ruta = dir.join(FICHERO);
    let bytes = match std::fs::read(&ruta) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Preferencias::default());
        }
        Err(error) => return Err(ErrorNucleo::lectura(&ruta, &error)),
    };
    match serde_json::from_slice::<Fichero>(&bytes) {
        Ok(fichero) if fichero.version <= VERSION => Ok(fichero.preferencias),
        _ => Err(ErrorNucleo::PreferenciasIlegibles {
            ruta: ruta.display().to_string(),
        }),
    }
}

/// Escribe las preferencias en `dir` de forma atómica: primero un temporal, que se fuerza a disco, y luego se renombra
/// encima. Así un corte a mitad, incluso de corriente, deja el fichero anterior entero.
pub fn guardar(dir: &Path, preferencias: &Preferencias) -> Result<(), ErrorNucleo> {
    use std::io::Write;

    let temporal = dir.join(TEMPORAL);
    let ruta = dir.join(FICHERO);
    let fichero = Fichero {
        version: VERSION,
        preferencias: preferencias.clone(),
    };
    let json = serde_json::to_vec_pretty(&fichero).map_err(|e| escritura(&ruta, &e))?;
    std::fs::create_dir_all(dir).map_err(|e| escritura(dir, &e))?;
    let mut salida = std::fs::File::create(&temporal).map_err(|e| escritura(&temporal, &e))?;
    salida
        .write_all(&json)
        .and_then(|()| salida.sync_all())
        .map_err(|e| escritura(&temporal, &e))?;
    drop(salida);
    std::fs::rename(&temporal, &ruta).map_err(|e| escritura(&ruta, &e))
}

fn escritura(ruta: &Path, error: &dyn std::fmt::Display) -> ErrorNucleo {
    ErrorNucleo::EscrituraFallida {
        ruta: ruta.display().to_string(),
        detalle: error.to_string(),
    }
}

/// Las preferencias de la app, compartidas por los comandos. El cerrojo hace que dos cambios a la vez no se pisen:
/// cada uno lee, cambia y guarda sin que otro se cuele en medio.
pub struct Almacen {
    dir: PathBuf,
    cerrojo: Mutex<()>,
}

impl Almacen {
    pub fn en(dir: PathBuf) -> Self {
        Self {
            dir,
            cerrojo: Mutex::new(()),
        }
    }

    pub fn leer(&self) -> Result<Preferencias, ErrorNucleo> {
        let _turno = self.cerrojo.lock().unwrap_or_else(|e| e.into_inner());
        cargar(&self.dir)
    }

    /// Lee, aplica `cambio` y guarda si ha cambiado algo. Si el fichero no se puede leer, no se cambia nada.
    pub fn cambiar<T>(
        &self,
        cambio: impl FnOnce(&mut Preferencias) -> Result<T, ErrorNucleo>,
    ) -> Result<T, ErrorNucleo> {
        let _turno = self.cerrojo.lock().unwrap_or_else(|e| e.into_inner());
        let antes = cargar(&self.dir)?;
        let mut preferencias = antes.clone();
        let resultado = cambio(&mut preferencias)?;
        if preferencias != antes {
            guardar(&self.dir, &preferencias)?;
        }
        Ok(resultado)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_fichero_son_vacias() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            cargar(&tmp.path().join("no-existe")).unwrap(),
            Preferencias::default()
        );
    }

    #[test]
    fn se_guardan_y_se_leen_igual_y_sin_temporal() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("datos");
        let mut preferencias = Preferencias::default();
        preferencias.anadir_carpeta(PathBuf::from(r"E:\Juegos\WoW"));
        preferencias.nombrar_instalacion(Path::new(r"E:\Juegos\WoW"), Some("Portátil".into()));
        preferencias.nombrar_cuenta(
            Path::new(r"E:\Juegos\WoW"),
            "_retail_",
            "1#1",
            Some("Alts".into()),
        );

        guardar(&dir, &preferencias).unwrap();
        // Una segunda vez renombra encima de un fichero que ya existe.
        guardar(&dir, &preferencias).unwrap();

        assert_eq!(cargar(&dir).unwrap(), preferencias);
        assert!(!dir.join(TEMPORAL).exists());
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join(FICHERO)).unwrap()).unwrap();
        assert_eq!(json["version"], VERSION);
        assert_eq!(json["carpetasManuales"][0], r"E:\Juegos\WoW");
    }

    #[test]
    fn un_fichero_ilegible_da_error_y_no_se_sobrescribe() {
        let tmp = tempfile::tempdir().unwrap();
        let ruta = tmp.path().join(FICHERO);
        std::fs::write(&ruta, "{ esto no es json").unwrap();
        let almacen = Almacen::en(tmp.path().to_path_buf());

        let lectura = almacen.leer().unwrap_err();
        let cambio = almacen
            .cambiar(|p| {
                p.anadir_carpeta(PathBuf::from("C:/WoW"));
                Ok(())
            })
            .unwrap_err();

        let esperado = ErrorNucleo::PreferenciasIlegibles {
            ruta: ruta.display().to_string(),
        };
        assert_eq!(lectura, esperado);
        assert_eq!(cambio, esperado);
        assert_eq!(std::fs::read_to_string(&ruta).unwrap(), "{ esto no es json");
    }

    #[test]
    fn un_fichero_de_una_version_posterior_no_se_toca() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(FICHERO), r#"{"version": 2}"#).unwrap();
        assert!(matches!(
            cargar(tmp.path()),
            Err(ErrorNucleo::PreferenciasIlegibles { .. })
        ));
    }

    #[test]
    fn la_misma_carpeta_escrita_de_otra_forma_no_se_repite() {
        let mut preferencias = Preferencias::default();
        assert!(preferencias.anadir_carpeta(PathBuf::from(r"E:\Juegos\WoW")));
        assert!(!preferencias.anadir_carpeta(PathBuf::from(r"e:\juegos\wow\")));
        assert_eq!(preferencias.carpetas_manuales.len(), 1);
    }

    #[test]
    fn los_nombres_se_ponen_cambian_y_borran() {
        let raiz = Path::new(r"E:\WoW");
        let mut preferencias = Preferencias::default();
        preferencias.nombrar_instalacion(raiz, Some("Casa".into()));
        preferencias.nombrar_instalacion(Path::new(r"e:\wow\"), Some("Portátil".into()));
        assert_eq!(
            preferencias.nombre_instalacion(raiz),
            Some("Portátil".into())
        );
        preferencias.nombrar_instalacion(raiz, None);
        assert_eq!(preferencias.nombre_instalacion(raiz), None);
    }

    #[test]
    fn las_cuentas_se_distinguen_por_instalacion_y_version() {
        let mut preferencias = Preferencias::default();
        preferencias.nombrar_cuenta(
            Path::new("C:/A"),
            "_retail_",
            "1#1",
            Some("Principal".into()),
        );
        assert_eq!(
            preferencias.nombre_cuenta(Path::new("C:/A"), "_retail_", "1#1"),
            Some("Principal".into())
        );
        assert_eq!(
            preferencias.nombre_cuenta(Path::new("C:/A"), "_forever_", "1#1"),
            None
        );
        assert_eq!(
            preferencias.nombre_cuenta(Path::new("C:/B"), "_retail_", "1#1"),
            None
        );
    }

    #[test]
    fn quitar_una_carpeta_borra_sus_nombres_y_solo_los_suyos() {
        let mut preferencias = Preferencias::default();
        for raiz in [r"E:\WoW", r"E:\WoW2"] {
            let raiz = Path::new(raiz);
            preferencias.anadir_carpeta(raiz.to_path_buf());
            preferencias.nombrar_instalacion(raiz, Some("x".into()));
            preferencias.nombrar_cuenta(raiz, "_retail_", "1#1", Some("y".into()));
        }

        preferencias.quitar_carpeta(Path::new(r"e:\wow"));

        assert_eq!(preferencias.carpetas_manuales, [PathBuf::from(r"E:\WoW2")]);
        assert_eq!(preferencias.nombres_instalacion.len(), 1);
        assert_eq!(preferencias.nombres_cuenta.len(), 1);
        assert!(
            preferencias
                .nombre_instalacion(Path::new(r"E:\WoW2"))
                .is_some()
        );
    }

    #[test]
    fn nombre_valido_recorta_vacia_y_limita() {
        assert_eq!(nombre_valido("  Casa  ").unwrap(), Some("Casa".into()));
        assert_eq!(nombre_valido("   ").unwrap(), None);
        let justo = "ñ".repeat(LARGO_MAXIMO_NOMBRE);
        assert_eq!(nombre_valido(&justo).unwrap(), Some(justo.clone()));
        let error = ErrorNucleo::NombreNoValido {
            maximo: LARGO_MAXIMO_NOMBRE,
        };
        assert_eq!(nombre_valido(&format!("{justo}a")), Err(error.clone()));
        assert_eq!(nombre_valido("Ca\nsa"), Err(error));
    }

    #[test]
    fn el_bom_se_recorta_como_en_la_interfaz() {
        assert_eq!(nombre_valido("\u{feff} \u{feff}").unwrap(), None);
    }

    #[test]
    fn un_cambio_que_no_cambia_nada_no_escribe() {
        let tmp = tempfile::tempdir().unwrap();
        let almacen = Almacen::en(tmp.path().to_path_buf());
        almacen
            .cambiar(|p| {
                p.quitar_carpeta(Path::new("C:/WoW"));
                Ok(())
            })
            .unwrap();
        assert!(!tmp.path().join(FICHERO).exists());
    }

    #[test]
    fn dos_cambios_a_la_vez_no_se_pisan() {
        let tmp = tempfile::tempdir().unwrap();
        let almacen = Almacen::en(tmp.path().to_path_buf());
        std::thread::scope(|hilos| {
            for i in 0..16 {
                let almacen = &almacen;
                hilos.spawn(move || {
                    almacen
                        .cambiar(|p| Ok(p.anadir_carpeta(PathBuf::from(format!("C:/WoW{i}")))))
                        .unwrap();
                });
            }
        });
        assert_eq!(almacen.leer().unwrap().carpetas_manuales.len(), 16);
    }

    #[test]
    fn el_almacen_guarda_lo_que_cambia() {
        let tmp = tempfile::tempdir().unwrap();
        let almacen = Almacen::en(tmp.path().join("datos"));
        let anadida = almacen
            .cambiar(|p| Ok(p.anadir_carpeta(PathBuf::from("C:/WoW"))))
            .unwrap();
        assert!(anadida);
        assert_eq!(
            almacen.leer().unwrap().carpetas_manuales,
            [PathBuf::from("C:/WoW")]
        );
    }
}
