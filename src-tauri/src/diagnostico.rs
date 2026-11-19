//! Registro de diagnóstico (#60). El núcleo escribe con la fachada `log` (`log::info!`, `log::warn!`…) sin saber
//! adónde va la línea; este módulo es el único logger: guarda las últimas líneas de la sesión en memoria, las emite a
//! la interfaz y, si el usuario lo ha encendido, las añade a un `.txt` acotado en la carpeta de datos de la app.
//!
//! El registro nunca tumba lo que registra: un fallo al escribir el `.txt` se calla.

use serde::Serialize;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Carpeta de los registros dentro de la carpeta de datos de la app.
pub const CARPETA_REGISTROS: &str = "registros";
/// El fichero que se está escribiendo.
const FICHERO: &str = "diagnostico.txt";
/// El anterior, cuando el actual se llenó. Con los dos, el registro no pasa de unos 2 MiB en disco.
const FICHERO_ANTERIOR: &str = "diagnostico.1.txt";
/// Tamaño a partir del cual se rota el fichero.
const MAXIMO_BYTES: u64 = 1024 * 1024;
/// Líneas de la sesión que se guardan en memoria para la pestaña Diagnóstico.
pub const MAXIMO_LINEAS: usize = 1000;
/// Solo se registran las líneas de la app: las de Tauri y sus crates son ruido y pueden llevar datos que no
/// controlamos.
const MODULO_APP: &str = env!("CARGO_CRATE_NAME");

/// Una línea del registro, tal como la ve la interfaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Linea {
    /// Número de orden en la sesión: la interfaz junta la carga inicial y los eventos sin repetir líneas.
    pub n: u64,
    /// Hora local con su desfase (`2026-11-19 10:32:05 +01:00`): quien lo lee es el jugador.
    pub instante: String,
    pub nivel: String,
    /// Módulo de la app sin el nombre del crate (`deteccion`, `operaciones::importar`).
    pub modulo: String,
    pub mensaje: String,
}

impl Linea {
    /// La línea como se guarda en el `.txt`.
    /// La línea como se guarda en el `.txt`. Un mensaje con saltos (el texto de un error del sistema) se queda en una
    /// sola línea: cada línea del fichero empieza por su hora.
    fn texto(&self) -> String {
        let mensaje = self.mensaje.replace(['\r', '\n'], " ");
        format!(
            "{} {:<5} [{}] {mensaje}\n",
            self.instante, self.nivel, self.modulo
        )
    }
}

/// Las últimas líneas de la sesión, con un tope.
struct Memoria {
    lineas: VecDeque<Linea>,
    maximo: usize,
}

impl Memoria {
    fn con_maximo(maximo: usize) -> Self {
        Self {
            lineas: VecDeque::with_capacity(maximo),
            maximo,
        }
    }

    fn anadir(&mut self, linea: Linea) {
        if self.lineas.len() == self.maximo {
            self.lineas.pop_front();
        }
        self.lineas.push_back(linea);
    }

    fn lineas(&self) -> Vec<Linea> {
        self.lineas.iter().cloned().collect()
    }
}

/// El `.txt` del registro. No se crea nada en disco hasta la primera línea; al llenarse, el actual pasa a ser el
/// anterior (pisando el que hubiera) y se empieza otro.
struct FicheroRotado {
    dir: PathBuf,
    maximo: u64,
    /// El fichero abierto y lo que ocupa.
    abierto: Option<(File, u64)>,
}

impl FicheroRotado {
    fn en(dir: PathBuf, maximo: u64) -> Self {
        Self {
            dir,
            maximo,
            abierto: None,
        }
    }

    fn escribir(&mut self, texto: &str) -> std::io::Result<()> {
        let largo = texto.len() as u64;
        // Si algo falla, `abierto` se queda vacío y la próxima línea vuelve a abrir el fichero.
        let (mut fichero, mut ocupado) = match self.abierto.take() {
            Some(abierto) => abierto,
            None => self.abrir()?,
        };
        // Un fichero vacío no se rota: una línea mayor que el máximo se escribe igual.
        if ocupado > 0 && ocupado + largo > self.maximo {
            // Se suelta el fichero antes de renombrarlo, para no depender de cómo lo comparta Windows.
            drop(fichero);
            // Si no se puede rotar (otro programa tiene abierto uno de los dos), se sigue en el actual: mejor un
            // fichero algo mayor que perder líneas. La siguiente línea lo vuelve a intentar.
            (fichero, ocupado) = self.rotar().or_else(|_| self.abrir())?;
        }
        fichero.write_all(texto.as_bytes())?;
        ocupado += largo;
        self.abierto = Some((fichero, ocupado));
        Ok(())
    }

    /// Abre (o crea) el fichero actual para añadir al final.
    fn abrir(&self) -> std::io::Result<(File, u64)> {
        std::fs::create_dir_all(&self.dir)?;
        let fichero = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join(FICHERO))?;
        let ocupado = fichero.metadata()?.len();
        Ok((fichero, ocupado))
    }

    /// El actual pasa a ser el anterior y se empieza uno vacío. En Windows, `rename` pisa el destino.
    fn rotar(&self) -> std::io::Result<(File, u64)> {
        std::fs::rename(self.dir.join(FICHERO), self.dir.join(FICHERO_ANTERIOR))?;
        self.abrir()
    }

    fn cerrar(&mut self) {
        self.abierto = None;
    }
}

/// Lo que hace la app con cada línea además de guardarla: emitirla a la interfaz. Es una función para poder probar
/// el registro sin Tauri.
pub type Emisor = Box<dyn Fn(&Linea) + Send + Sync>;

struct Estado {
    siguiente: u64,
    memoria: Memoria,
    fichero: FicheroRotado,
}

/// El logger de la app.
pub struct Registro {
    estado: Mutex<Estado>,
    /// El interruptor «Guardar registro de diagnóstico».
    guardar: AtomicBool,
    emisor: OnceLock<Emisor>,
    dir: PathBuf,
}

impl Registro {
    /// `dir`: la carpeta de los registros. `guardar`: el interruptor guardado en las preferencias.
    pub fn en(dir: PathBuf, guardar: bool) -> Self {
        Self::con_limites(dir, guardar, MAXIMO_LINEAS, MAXIMO_BYTES)
    }

    fn con_limites(dir: PathBuf, guardar: bool, lineas: usize, bytes: u64) -> Self {
        Self {
            estado: Mutex::new(Estado {
                siguiente: 0,
                memoria: Memoria::con_maximo(lineas),
                fichero: FicheroRotado::en(dir.clone(), bytes),
            }),
            guardar: AtomicBool::new(guardar),
            emisor: OnceLock::new(),
            dir,
        }
    }

    /// Engancha la salida hacia la interfaz. Solo la primera vez cuenta.
    pub fn conectar(&self, emisor: Emisor) {
        let _ = self.emisor.set(emisor);
    }

    pub fn carpeta(&self) -> &Path {
        &self.dir
    }

    pub fn guarda(&self) -> bool {
        self.guardar.load(Ordering::Relaxed)
    }

    /// Enciende o apaga el `.txt`. Al apagarlo se cierra el fichero; lo ya escrito se queda.
    pub fn cambiar(&self, guardar: bool) {
        self.guardar.store(guardar, Ordering::Relaxed);
        if !guardar {
            self.bloquear().fichero.cerrar();
        }
    }

    /// Las líneas de la sesión que siguen en memoria, de la más antigua a la más nueva.
    pub fn lineas(&self) -> Vec<Linea> {
        self.bloquear().memoria.lineas()
    }

    fn anotar(&self, nivel: log::Level, modulo: &str, mensaje: String) {
        let linea = {
            let mut estado = self.bloquear();
            let linea = Linea {
                n: estado.siguiente,
                instante: instante_local(),
                nivel: nivel.to_string(),
                modulo: modulo.to_owned(),
                mensaje,
            };
            estado.siguiente += 1;
            if self.guarda() {
                // Un disco lleno o una carpeta sin permiso no deben cortar la operación que se está registrando. Se
                // escribe con el cerrojo tomado para que las líneas no se mezclen; la carpeta es la de datos de la app,
                // en el disco local.
                let _ = estado.fichero.escribir(&linea.texto());
            }
            estado.memoria.anadir(linea.clone());
            linea
        };
        // Fuera del cerrojo: si emitir volviera a registrar, no se bloquearía.
        if let Some(emitir) = self.emisor.get() {
            emitir(&linea);
        }
    }

    /// Un `panic` con el cerrojo tomado no debe dejar la app sin registro: se sigue con lo que hubiera.
    fn bloquear(&self) -> std::sync::MutexGuard<'_, Estado> {
        self.estado.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl log::Log for Registro {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info && modulo_de(metadata.target()).is_some()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        if let Some(modulo) = modulo_de(record.target()) {
            self.anotar(record.level(), modulo, record.args().to_string());
        }
    }

    fn flush(&self) {}
}

/// El módulo de la app sin el nombre del crate, o `None` si la línea no es de la app.
fn modulo_de(target: &str) -> Option<&str> {
    let resto = target.strip_prefix(MODULO_APP)?;
    if resto.is_empty() {
        return Some(MODULO_APP);
    }
    resto.strip_prefix("::")
}

/// Hora local con su desfase. Si Windows no da el desfase, UTC (`+00:00`).
fn instante_local() -> String {
    let ahora =
        time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let desfase = ahora.offset();
    let signo = if desfase.is_negative() { '-' } else { '+' };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} {signo}{:02}:{:02}",
        ahora.year(),
        u8::from(ahora.month()),
        ahora.day(),
        ahora.hour(),
        ahora.minute(),
        ahora.second(),
        desfase.whole_hours().unsigned_abs(),
        desfase.minutes_past_hour().unsigned_abs()
    )
}

static REGISTRO: OnceLock<Registro> = OnceLock::new();

/// Instala el registro como logger de la app. Solo la primera llamada cuenta; las demás devuelven el ya instalado.
pub fn instalar(registro: Registro) -> &'static Registro {
    let instalado = REGISTRO.get_or_init(|| registro);
    if log::set_logger(instalado).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
    instalado
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::Log;
    use std::sync::Arc;

    fn linea(registro: &Registro, mensaje: &str) {
        registro.log(
            &log::Record::builder()
                .level(log::Level::Info)
                .target(&format!("{MODULO_APP}::deteccion"))
                .args(format_args!("{mensaje}"))
                .build(),
        );
    }

    fn leer(dir: &Path, nombre: &str) -> String {
        std::fs::read_to_string(dir.join(nombre)).unwrap_or_default()
    }

    #[test]
    fn la_memoria_guarda_solo_las_ultimas() {
        let dir = tempfile::tempdir().unwrap();
        let registro = Registro::con_limites(dir.path().join("r"), false, 3, MAXIMO_BYTES);
        for i in 0..5 {
            linea(&registro, &format!("m{i}"));
        }
        let lineas = registro.lineas();
        let mensajes: Vec<_> = lineas.iter().map(|l| l.mensaje.as_str()).collect();
        assert_eq!(mensajes, ["m2", "m3", "m4"]);
        assert_eq!(lineas[0].n, 2);
        assert_eq!(lineas[0].modulo, "deteccion");
        assert_eq!(lineas[0].nivel, "INFO");
    }

    #[test]
    fn apagado_no_toca_el_disco() {
        let dir = tempfile::tempdir().unwrap();
        let carpeta = dir.path().join("registros");
        let registro = Registro::en(carpeta.clone(), false);
        linea(&registro, "hola");
        assert!(!carpeta.exists());
        assert_eq!(registro.lineas().len(), 1);
    }

    #[test]
    fn encendido_escribe_y_al_apagar_deja_de_escribir() {
        let dir = tempfile::tempdir().unwrap();
        let carpeta = dir.path().join("registros");
        let registro = Registro::en(carpeta.clone(), true);
        linea(&registro, "uno");
        registro.cambiar(false);
        linea(&registro, "dos");
        registro.cambiar(true);
        linea(&registro, "tres");
        let texto = leer(&carpeta, FICHERO);
        assert!(texto.contains("[deteccion] uno\n"), "{texto}");
        assert!(!texto.contains("dos"));
        assert!(texto.contains("tres"));
        assert_eq!(registro.lineas().len(), 3);
    }

    #[test]
    fn al_llenarse_rota_y_pisa_el_anterior() {
        let dir = tempfile::tempdir().unwrap();
        let carpeta = dir.path().join("registros");
        let mut fichero = FicheroRotado::en(carpeta.clone(), 10);
        fichero.escribir("aaaaaa\n").unwrap();
        fichero.escribir("bbbbbb\n").unwrap();
        assert_eq!(leer(&carpeta, FICHERO_ANTERIOR), "aaaaaa\n");
        assert_eq!(leer(&carpeta, FICHERO), "bbbbbb\n");
        fichero.escribir("cccccc\n").unwrap();
        assert_eq!(leer(&carpeta, FICHERO_ANTERIOR), "bbbbbb\n");
        assert_eq!(leer(&carpeta, FICHERO), "cccccc\n");
    }

    #[test]
    fn una_linea_mayor_que_el_maximo_se_escribe_igual_sin_pisar_el_anterior() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FICHERO_ANTERIOR), "anterior\n").unwrap();
        let mut fichero = FicheroRotado::en(dir.path().to_path_buf(), 4);
        fichero.escribir("larguisima\n").unwrap();
        assert_eq!(leer(dir.path(), FICHERO), "larguisima\n");
        // Rotar un fichero vacío cambiaría el anterior por nada.
        assert_eq!(leer(dir.path(), FICHERO_ANTERIOR), "anterior\n");
    }

    #[test]
    fn si_no_se_puede_rotar_sigue_en_el_actual() {
        let dir = tempfile::tempdir().unwrap();
        // Una carpeta no vacía donde iría el anterior: `rename` falla, como con el fichero abierto por otro programa.
        let estorbo = dir.path().join(FICHERO_ANTERIOR);
        std::fs::create_dir(&estorbo).unwrap();
        std::fs::write(estorbo.join("x"), "").unwrap();
        let mut fichero = FicheroRotado::en(dir.path().to_path_buf(), 10);
        fichero.escribir("aaaaaa\n").unwrap();
        fichero.escribir("bbbbbb\n").unwrap();
        assert_eq!(leer(dir.path(), FICHERO), "aaaaaa\nbbbbbb\n");
    }

    #[test]
    fn un_mensaje_con_saltos_queda_en_una_linea() {
        let linea = Linea {
            n: 0,
            instante: "t".into(),
            nivel: "WARN".into(),
            modulo: "m".into(),
            mensaje: "uno\r\ndos".into(),
        };
        assert_eq!(linea.texto(), "t WARN  [m] uno  dos\n");
    }

    #[test]
    fn sigue_el_fichero_que_ya_habia() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FICHERO), "previa\n").unwrap();
        let mut fichero = FicheroRotado::en(dir.path().to_path_buf(), MAXIMO_BYTES);
        fichero.escribir("nueva\n").unwrap();
        assert_eq!(leer(dir.path(), FICHERO), "previa\nnueva\n");
    }

    #[test]
    fn una_carpeta_que_no_se_puede_crear_no_corta_nada() {
        let dir = tempfile::tempdir().unwrap();
        // Un fichero donde debería ir la carpeta: `create_dir_all` falla.
        let estorbo = dir.path().join("registros");
        std::fs::write(&estorbo, "").unwrap();
        let registro = Registro::en(estorbo, true);
        linea(&registro, "sigue");
        assert_eq!(registro.lineas().len(), 1);
    }

    #[test]
    fn solo_entran_las_lineas_de_la_app_desde_info() {
        let dir = tempfile::tempdir().unwrap();
        let registro = Registro::en(dir.path().join("r"), false);
        let de_la_app = format!("{MODULO_APP}::operaciones");
        let casos = [
            (log::Level::Info, "tauri::manager", false),
            (log::Level::Info, "wow_profile_manager_libx::a", false),
            (log::Level::Debug, de_la_app.as_str(), false),
            (log::Level::Warn, de_la_app.as_str(), true),
            (log::Level::Info, MODULO_APP, true),
        ];
        for (nivel, target, entra) in casos {
            let metadata = log::Metadata::builder().level(nivel).target(target).build();
            assert_eq!(registro.enabled(&metadata), entra, "{nivel} {target}");
        }
    }

    #[test]
    fn emite_cada_linea_fuera_del_cerrojo() {
        let dir = tempfile::tempdir().unwrap();
        let registro = Arc::new(Registro::en(dir.path().join("r"), false));
        let vistas = Arc::new(Mutex::new(Vec::new()));
        let (copia, de_vuelta) = (Arc::clone(&vistas), Arc::clone(&registro));
        registro.conectar(Box::new(move |l| {
            // Leer el registro desde el emisor no debe bloquear.
            let total = de_vuelta.lineas().len();
            copia.lock().unwrap().push((l.mensaje.clone(), total));
        }));
        linea(&registro, "a");
        linea(&registro, "b");
        assert_eq!(
            *vistas.lock().unwrap(),
            [("a".to_owned(), 1), ("b".to_owned(), 2)]
        );
    }

    #[test]
    fn el_instante_lleva_fecha_hora_y_desfase() {
        let instante = instante_local();
        let partes: Vec<_> = instante.split(' ').collect();
        assert_eq!(partes.len(), 3, "{instante}");
        assert_eq!(partes[0].len(), 10);
        assert_eq!(partes[1].len(), 8);
        assert!(partes[2].starts_with(['+', '-']) && partes[2].len() == 6);
    }
}
