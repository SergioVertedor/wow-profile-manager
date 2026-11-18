//! Historial (spec 022, `docs/flujos.md`): listar las operaciones, saber dónde está la carpeta de una y borrarla. Lee
//! el `WTF` para decir qué quedó sin devolver, pero nunca lo escribe.

use super::deshacer::se_puede_deshacer;
use super::{
    Destino, Diario, Estado, Operaciones, PaqueteImportado, Tipo, en_disco, es_enlace,
    hash_en_disco, leer,
};
use crate::disco::subcarpetas;
use crate::error::ErrorNucleo;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Lo que devuelve `listar`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Historial {
    /// De la más nueva a la más antigua.
    operaciones: Vec<Resumen>,
    /// Lo que ocupan todas las carpetas de operaciones, ilegibles incluidas.
    bytes: u64,
    /// Carpetas cuyo `operacion.json` no se entiende: se cuentan aparte y no se tocan.
    ilegibles: Vec<Ilegible>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Ilegible {
    id: String,
    bytes: u64,
}

/// Una operación tal como la enseña el historial.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Resumen {
    /// El nombre de su carpeta.
    id: String,
    tipo: Tipo,
    creada: String,
    terminada: Option<String>,
    estado: Estado,
    error: Option<String>,
    destino: Destino,
    /// En una importación, el perfil importado.
    paquete: Option<PaqueteImportado>,
    deshace: Option<String>,
    deshecha_por: Option<String>,
    /// Un `deshacer` cuya original ya se borró del historial.
    deshace_borrada: bool,
    /// Cuántos ficheros toca.
    ficheros: usize,
    /// Lo que ocupa su carpeta.
    bytes: u64,
    /// Si se le puede ofrecer «Deshacer»: una eliminación o una importación `completa`.
    deshacible: bool,
    /// Una importación cortada a mitad: se le ofrece «Restaurar» y bloquea las demás operaciones (spec 025).
    a_medias: bool,
    /// En una `fallida-sin-restaurar` o una importación a medias, los ficheros que hoy no están como antes y dónde está
    /// su copia.
    sin_devolver: Vec<SinDevolver>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SinDevolver {
    ruta: String,
    /// `None` si el fichero no existía antes: entonces lo que sobra es el que hay ahora.
    copia: Option<PathBuf>,
}

impl Operaciones {
    /// Todas las operaciones, de la más nueva a la más antigua, con lo que ocupan. Solo lee.
    pub fn listar(&self) -> Historial {
        let carpetas = subcarpetas(&self.dir).unwrap_or_default();
        let ids: HashSet<&str> = carpetas.iter().map(|c| c.nombre.as_str()).collect();
        let mut historial = Historial::default();
        for carpeta in &carpetas {
            let bytes = bytes_de(&carpeta.ruta);
            historial.bytes += bytes;
            let id = carpeta.nombre.clone();
            match leer(&carpeta.ruta) {
                Some(diario) => {
                    historial
                        .operaciones
                        .push(resumir(&carpeta.ruta, id, diario, bytes, &ids))
                }
                None => historial.ilegibles.push(Ilegible { id, bytes }),
            }
        }
        historial
            .operaciones
            .sort_by(|a, b| (&b.creada, &b.id).cmp(&(&a.creada, &a.id)));
        historial
    }

    /// La carpeta de la operación `id`. El id solo vale si es el nombre de una carpeta de la lista: así un `..` o una
    /// ruta nunca llegan a abrirse ni a borrarse.
    pub fn carpeta_de(&self, id: &str) -> Result<PathBuf, ErrorNucleo> {
        // Una carpeta de operación nunca es un enlace: si lo fuera, `leer` y abrir la seguirían fuera de los datos.
        subcarpetas(&self.dir)?
            .into_iter()
            .find(|c| c.nombre == id && !es_enlace(&c.ruta))
            .map(|c| c.ruta)
            .ok_or(ErrorNucleo::OperacionNoEncontrada {})
    }

    /// Borra la carpeta de una operación terminada y con ella la posibilidad de deshacerla (`docs/flujos.md`,
    /// «Historial»). Una `fallida-sin-restaurar` solo si el usuario confirma que ya restauró a mano: sus copias pueden
    /// ser lo único que queda del estado anterior.
    pub fn borrar(&self, id: &str, restaurada_a_mano: bool) -> Result<(), ErrorNucleo> {
        let _turno = self.cerrojo.lock().unwrap_or_else(|e| e.into_inner());
        let dir = self.carpeta_de(id)?;
        let diario = leer(&dir).ok_or(ErrorNucleo::OperacionNoEncontrada {})?;
        if !es_final(diario.estado) {
            return Err(ErrorNucleo::OperacionEnCurso {});
        }
        if diario.estado == Estado::FallidaSinRestaurar && !restaurada_a_mano {
            return Err(ErrorNucleo::RestauracionManualSinConfirmar {});
        }
        // `remove_dir_all` no sigue enlaces. No es atómico: si falla a mitad, lo que quede sale como ilegible y se
        // puede volver a borrar a mano; el `WTF` no se toca.
        std::fs::remove_dir_all(&dir).map_err(|e| ErrorNucleo::escritura(&dir, &e))
    }
}

fn es_final(estado: Estado) -> bool {
    !matches!(
        estado,
        Estado::Preparando | Estado::Copiada | Estado::Escribiendo
    )
}

fn resumir(dir: &Path, id: String, diario: Diario, bytes: u64, ids: &HashSet<&str>) -> Resumen {
    let a_medias = diario.tipo == Tipo::Importacion && diario.estado == Estado::Escribiendo;
    let sin_devolver = if a_medias || diario.estado == Estado::FallidaSinRestaurar {
        sin_devolver(dir, &diario)
    } else {
        Vec::new()
    };
    let deshace_borrada = diario
        .deshace
        .as_deref()
        .is_some_and(|original| !ids.contains(original));
    Resumen {
        id,
        deshacible: se_puede_deshacer(&diario),
        a_medias,
        ficheros: diario.cambios.len(),
        bytes,
        sin_devolver,
        deshace_borrada,
        tipo: diario.tipo,
        creada: diario.creada,
        terminada: diario.terminada,
        estado: diario.estado,
        error: diario.error,
        destino: diario.destino,
        paquete: diario.paquete,
        deshace: diario.deshace,
        deshecha_por: diario.deshecha_por,
    }
}

/// Los ficheros que hoy no están como antes de la operación, medidos contra el disco: el texto del error dice lo que
/// faltaba cuando falló, pero el usuario pudo devolver algunos a mano después.
fn sin_devolver(dir: &Path, diario: &Diario) -> Vec<SinDevolver> {
    diario
        .cambios
        .iter()
        .filter_map(|cambio| {
            let ruta = en_disco(diario, cambio).ok();
            let devuelto = ruta
                .as_deref()
                .and_then(|r| hash_en_disco(r).ok())
                .is_some_and(|actual| actual == cambio.antes.sha256);
            (!devuelto).then(|| SinDevolver {
                ruta: ruta.map_or_else(|| cambio.ruta.clone(), |r| r.display().to_string()),
                copia: cambio.antes.copia.as_ref().map(|c| dir.join(c)),
            })
        })
        .collect()
}

/// Lo que ocupan los ficheros de `carpeta` y sus subcarpetas, sin seguir enlaces. Lo que no se puede leer no cuenta.
fn bytes_de(carpeta: &Path) -> u64 {
    let mut pendientes = vec![carpeta.to_path_buf()];
    let mut total = 0;
    while let Some(actual) = pendientes.pop() {
        let Ok(entradas) = std::fs::read_dir(&actual) else {
            continue;
        };
        for entrada in entradas.flatten() {
            // `file_type` de la entrada no sigue enlaces: un enlace no es ni carpeta ni fichero aquí.
            match entrada.file_type() {
                Ok(tipo) if tipo.is_dir() => pendientes.push(entrada.path()),
                Ok(tipo) if tipo.is_file() => {
                    total += entrada.metadata().map_or(0, |m| m.len());
                }
                _ => {}
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::super::guardar;
    use super::super::tests::{ADDONS_TXT, Escenario, cortada, diario_de, escenario};
    use super::*;
    use std::fs::{create_dir_all, write};

    fn eliminado(e: &Escenario) -> String {
        e.operaciones
            .eliminar_personaje(&e.personaje, || false)
            .unwrap()
    }

    /// Una eliminación y su deshacer. La eliminación se fecha en el pasado: las dos caen en el mismo segundo.
    fn eliminado_y_deshecho(e: &Escenario) -> (String, String) {
        let id = eliminado(e);
        let mut diario = diario_de(&e.datos, &id);
        diario.creada = "2000-01-01T00:00:00Z".into();
        guardar(&e.datos.join(&id), &diario).unwrap();
        let plan = e.operaciones.plan_deshacer(&id).unwrap();
        let nuevo = e.operaciones.deshacer(&id, &plan.huella, || false).unwrap();
        (id, nuevo)
    }

    #[test]
    fn lista_de_la_mas_nueva_a_la_mas_antigua_con_lo_que_ocupan() {
        let e = escenario();
        let (id, nuevo) = eliminado_y_deshecho(&e);

        let historial = e.operaciones.listar();

        let ids: Vec<&str> = historial
            .operaciones
            .iter()
            .map(|o| o.id.as_str())
            .collect();
        assert_eq!(ids, [nuevo.as_str(), id.as_str()]);
        let eliminacion = &historial.operaciones[1];
        assert_eq!(eliminacion.estado, Estado::Deshecha);
        assert!(!eliminacion.deshacible);
        assert_eq!(eliminacion.ficheros, 2);
        assert!(eliminacion.bytes > ADDONS_TXT.len() as u64);
        let total: u64 = historial.operaciones.iter().map(|o| o.bytes).sum();
        assert_eq!(historial.bytes, total);
        assert!(historial.ilegibles.is_empty());
    }

    #[test]
    fn una_eliminacion_completa_se_puede_deshacer() {
        let e = escenario();
        eliminado(&e);
        let historial = e.operaciones.listar();
        assert!(historial.operaciones[0].deshacible);
        assert_eq!(
            historial.operaciones[0].destino.carpeta_personaje,
            "Goldberg-Soulreaper"
        );
    }

    #[test]
    fn un_diario_ilegible_se_cuenta_aparte_con_lo_que_ocupa() {
        let e = escenario();
        let dir = e.datos.join("20261118-093549-0000abcd");
        create_dir_all(&dir).unwrap();
        write(dir.join("operacion.json"), "{ roto").unwrap();

        let historial = e.operaciones.listar();

        assert!(historial.operaciones.is_empty());
        assert_eq!(historial.ilegibles[0].id, "20261118-093549-0000abcd");
        assert_eq!(historial.bytes, 6);
    }

    /// Lo que queda sin devolver se mide al listar: si el usuario devuelve un fichero a mano, deja de salir.
    #[test]
    fn una_sin_restaurar_dice_que_falta_y_donde_esta_su_copia_mientras_falte() {
        let e = escenario();
        let id = cortada(&e, Estado::Escribiendo);
        let prat = e
            .personaje
            .carpeta()
            .join("SavedVariables")
            .join("Prat-3.0.lua");
        write(&prat, "nuevo").unwrap();
        e.operaciones.recuperar(|| false);

        let historial = e.operaciones.listar();
        let operacion = &historial.operaciones[0];
        assert_eq!(operacion.estado, Estado::FallidaSinRestaurar);
        assert_eq!(operacion.sin_devolver.len(), 1);
        assert_eq!(operacion.sin_devolver[0].ruta, prat.display().to_string());
        let copia = operacion.sin_devolver[0].copia.clone().unwrap();
        assert_eq!(copia, e.datos.join(&id).join("copias/1"));

        std::fs::copy(&copia, &prat).unwrap();
        assert!(
            e.operaciones.listar().operaciones[0]
                .sin_devolver
                .is_empty()
        );
    }

    #[test]
    fn borrar_libera_su_carpeta_y_el_deshacer_dice_que_la_original_ya_no_esta() {
        let e = escenario();
        let (id, nuevo) = eliminado_y_deshecho(&e);

        e.operaciones.borrar(&id, false).unwrap();

        assert!(!e.datos.join(&id).exists());
        let historial = e.operaciones.listar();
        assert_eq!(historial.operaciones.len(), 1);
        assert_eq!(historial.operaciones[0].id, nuevo);
        assert!(historial.operaciones[0].deshace_borrada);
    }

    #[test]
    fn no_se_borra_una_a_medias_ni_una_sin_restaurar_sin_confirmarlo() {
        let e = escenario();
        let id = cortada(&e, Estado::Escribiendo);
        assert_eq!(
            e.operaciones.borrar(&id, true),
            Err(ErrorNucleo::OperacionEnCurso {})
        );

        let mut diario = diario_de(&e.datos, &id);
        diario.terminar(Estado::FallidaSinRestaurar, Some("x".into()));
        guardar(&e.datos.join(&id), &diario).unwrap();
        assert_eq!(
            e.operaciones.borrar(&id, false),
            Err(ErrorNucleo::RestauracionManualSinConfirmar {})
        );
        assert!(e.datos.join(&id).exists());

        e.operaciones.borrar(&id, true).unwrap();
        assert!(!e.datos.join(&id).exists());
    }

    #[test]
    fn un_id_que_no_es_una_carpeta_de_la_lista_no_se_usa() {
        let e = escenario();
        eliminado(&e);
        for id in ["..", "../datos", "", "20261118-093549-0000abcd"] {
            assert_eq!(
                e.operaciones.carpeta_de(id),
                Err(ErrorNucleo::OperacionNoEncontrada {}),
                "{id}"
            );
            assert_eq!(
                e.operaciones.borrar(id, true),
                Err(ErrorNucleo::OperacionNoEncontrada {}),
                "{id}"
            );
        }
        assert!(e.datos.exists());
    }
}
