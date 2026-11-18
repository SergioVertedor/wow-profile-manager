//! Sistema (ver `docs/arquitectura.md`): si el juego está en marcha. Con WoW abierto no se escribe en `WTF`: al salir,
//! el cliente reescribe sus ficheros encima (`docs/flujos.md`).

/// Forever y Retail se lanzan con el mismo ejecutable [V], así que no se distingue la versión: cualquiera bloquea.
const EJECUTABLE: &str = "Wow.exe";

/// Si hay algún `Wow.exe` en marcha. Pregunta a `tasklist.exe` de `System32` (el núcleo prohíbe `unsafe`, que haría
/// falta para la API de procesos). Si no se puede preguntar, responde que sí: es mejor no escribir que escribir con
/// el juego abierto.
pub fn juego_en_marcha() -> bool {
    salida_de_tasklist().is_none_or(|csv| hay_ejecutable(&csv))
}

#[cfg(windows)]
fn salida_de_tasklist() -> Option<String> {
    use std::os::windows::process::CommandExt;

    /// Sin ventana de consola: si no, parpadea una al preguntar.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let tasklist = std::path::Path::new(&system_root)
        .join("System32")
        .join("tasklist.exe");
    let salida = std::process::Command::new(tasklist)
        .args([
            "/FI",
            &format!("IMAGENAME eq {EJECUTABLE}"),
            "/FO",
            "CSV",
            "/NH",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    salida
        .status
        .success()
        .then(|| String::from_utf8_lossy(&salida.stdout).into_owned())
}

#[cfg(not(windows))]
fn salida_de_tasklist() -> Option<String> {
    None
}

/// En CSV, cada proceso es una línea que empieza por su nombre entre comillas. Sin procesos, `tasklist` escribe un
/// aviso en el idioma de Windows, que no empieza por comillas.
fn hay_ejecutable(csv: &str) -> bool {
    let buscado = format!("\"{EJECUTABLE}\"").to_lowercase();
    csv.lines()
        .any(|linea| linea.trim_start().to_lowercase().starts_with(&buscado))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconoce_el_juego_en_la_salida_de_tasklist() {
        assert!(hay_ejecutable(
            "\"Wow.exe\",\"1234\",\"Console\",\"1\",\"2.000.000 K\"\r\n"
        ));
        assert!(hay_ejecutable(
            "\"WOW.EXE\",\"1\",\"Console\",\"1\",\"1 K\""
        ));
    }

    #[test]
    fn el_aviso_sin_procesos_y_otros_ejecutables_no_cuentan() {
        assert!(!hay_ejecutable(
            "INFO: No hay tareas ejecutándose que coincidan con los criterios especificados.\r\n"
        ));
        assert!(!hay_ejecutable(
            "\"wow-profile-manager.exe\",\"9\",\"Console\",\"1\",\"1 K\""
        ));
        assert!(!hay_ejecutable(""));
    }

    #[test]
    #[cfg(windows)]
    fn tasklist_responde_en_este_equipo() {
        assert!(salida_de_tasklist().is_some());
    }
}
