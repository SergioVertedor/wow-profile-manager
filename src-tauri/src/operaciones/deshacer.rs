//! Deshacer (spec 022, `docs/flujos.md`, «Deshacer»): devolver la carpeta de un personaje exactamente como estaba antes
//! de eliminarla. Es otra operación, de tipo `deshacer`, con el mismo orden de escritura: primero copia lo que haya ahora
//! (el juego pudo volver a crear la carpeta) y después restaura. La original pasa a `deshecha`.

use super::{
    Accion, Antes, CARPETA_ACCOUNT, CARPETA_WTF, Cambio, Despues, Destino, Diario, ESQUEMA, Estado,
    MAXIMO_CAMBIOS, Operaciones, Tipo, ahora_rfc3339, comprobar, contenido, copia_n, ejecutar,
    en_disco, enlace, es_enlace, ficheros_de, guardar, leer, relativa_con_barras, ruta_segura,
    sha256, sigue_como_antes,
};
use crate::deteccion::producto_de;
use crate::disco::{clave_de_ruta, subcarpetas};
use crate::error::ErrorNucleo;
use crate::escritura::escribir_atomico;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Lo que la interfaz enseña antes de confirmar un deshacer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDeshacer {
    /// Ficheros que vuelven desde la copia.
    pub vuelven: usize,
    /// Ficheros que hay ahora y antes no estaban: se quitan.
    pub sobran: usize,
    /// Los que existen ahora y cambian (se pisan o se quitan): lo que el juego guardó en ellos desde entonces se pierde.
    pub cambian: Vec<String>,
    /// Huella del estado actual de esos ficheros: deshacer solo sigue si no ha cambiado desde que se enseñó.
    pub huella: String,
}

/// Todo lo que hace falta para deshacer, calculado igual para el plan y para escribir.
struct Preparado {
    original: Diario,
    cambios: Vec<Cambio>,
    /// Por cada cambio, la copia de la original de la que sale (`restaurar`) o `None` (`borrar`).
    fuentes: Vec<Option<PathBuf>>,
}

impl Preparado {
    fn contar(&self, accion: Accion) -> usize {
        self.cambios.iter().filter(|c| c.accion == accion).count()
    }

    fn plan(&self) -> PlanDeshacer {
        PlanDeshacer {
            vuelven: self.contar(Accion::Restaurar),
            sobran: self.contar(Accion::Borrar),
            cambian: self
                .cambios
                .iter()
                .filter(|c| c.antes.existia)
                .map(|c| c.ruta.clone())
                .collect(),
            huella: huella(&self.cambios),
        }
    }
}

impl Operaciones {
    /// Qué haría deshacer la operación `id`, con todas sus comprobaciones. Solo lee.
    pub fn plan_deshacer(&self, id: &str) -> Result<PlanDeshacer, ErrorNucleo> {
        Ok(self.preparar_deshacer(id)?.plan())
    }

    /// Deshace la operación `id` si el disco sigue como cuando se calculó `huella_vista`. Devuelve el id del deshacer.
    pub fn deshacer(
        &self,
        id: &str,
        huella_vista: &str,
        juego_en_marcha: impl Fn() -> bool,
    ) -> Result<String, ErrorNucleo> {
        let _turno = self.cerrojo.lock().unwrap_or_else(|e| e.into_inner());
        if juego_en_marcha() {
            return Err(ErrorNucleo::JuegoAbierto {});
        }
        // Como al eliminar: una pendiente se termina antes, para que el siguiente arranque no la restaure encima.
        self.recuperar_sin_cerrojo(&juego_en_marcha);
        let preparado = self.preparar_deshacer(id)?;
        if huella(&preparado.cambios) != huella_vista {
            return Err(ErrorNucleo::PlanCaducado {});
        }
        let (nuevo, dir) = self.nueva_carpeta()?;
        let diario = Diario {
            esquema: ESQUEMA,
            id: nuevo,
            tipo: Tipo::Deshacer,
            creada: ahora_rfc3339(),
            terminada: None,
            estado: Estado::Preparando,
            deshace: Some(id.to_owned()),
            deshecha_por: None,
            error: None,
            destino: preparado.original.destino,
            cambios: preparado.cambios,
        };
        if let Err(error) = guardar(&dir, &diario) {
            // Aún no hay copias ni se ha tocado el `WTF`.
            std::fs::remove_dir_all(&dir).ok();
            return Err(error);
        }
        let fuentes = preparado.fuentes;
        let hecho = ejecutar(&dir, diario, &juego_en_marcha, |diario| {
            aplicar(diario, &fuentes)
        })?;
        // Si se corta aquí, el siguiente arranque la marca (`recuperar`).
        self.marcar_deshecha(id, &hecho)?;
        Ok(hecho)
    }

    fn preparar_deshacer(&self, id: &str) -> Result<Preparado, ErrorNucleo> {
        let original_dir = self.carpeta_de(id)?;
        let original = leer(&original_dir).ok_or(ErrorNucleo::OperacionNoEncontrada {})?;
        if original.tipo != Tipo::Eliminacion || original.estado != Estado::Completa {
            return Err(ErrorNucleo::NoDeshacible {});
        }
        mismo_juego(&original.destino)?;
        if let Some(posterior) = self.posterior_que_la_toca(id, &original)? {
            return Err(ErrorNucleo::OperacionPosterior { id: posterior });
        }
        let (carpeta, base) = carpeta_del_personaje(&original)?;
        let actuales = ficheros_actuales(&carpeta, &base)?;
        construir(&original_dir, original, actuales)
    }

    /// Una operación posterior `completa` en la misma versión del juego que tocó alguno de los mismos ficheros. Se
    /// deshacen de la más nueva a la más antigua: así nunca se pisa en silencio una más reciente. Un `deshacer` no
    /// cuenta: solo devolvió un estado anterior, y lo que pisaría sale en `cambian` del plan.
    fn posterior_que_la_toca(
        &self,
        id: &str,
        original: &Diario,
    ) -> Result<Option<String>, ErrorNucleo> {
        let rutas: HashSet<String> = original
            .cambios
            .iter()
            .map(|c| c.ruta.to_lowercase())
            .collect();
        let raiz = clave_de_ruta(&original.destino.raiz_version);
        Ok(subcarpetas(&self.dir)?
            .into_iter()
            .filter_map(|c| leer(&c.ruta).map(|d| (c.nombre, d)))
            .find(|(nombre, d)| {
                (&d.creada, nombre.as_str()) > (&original.creada, id)
                    && d.estado == Estado::Completa
                    && d.tipo != Tipo::Deshacer
                    && clave_de_ruta(&d.destino.raiz_version) == raiz
                    && d.cambios
                        .iter()
                        .any(|c| rutas.contains(&c.ruta.to_lowercase()))
            })
            .map(|(nombre, _)| nombre))
    }
}

/// La raíz de la versión tiene que seguir existiendo y ser del mismo producto (`docs/flujos.md`, «Mismo juego»).
fn mismo_juego(destino: &Destino) -> Result<(), ErrorNucleo> {
    let raiz = &destino.raiz_version;
    let producto = if raiz.is_dir() {
        producto_de(raiz)?
    } else {
        None
    };
    if producto.map(|p| p.codigo()) == Some(destino.producto.as_str()) {
        Ok(())
    } else {
        Err(ErrorNucleo::JuegoDistinto {
            raiz: raiz.display().to_string(),
        })
    }
}

/// La carpeta del personaje y su ruta relativa (`WTF/Account/<cuenta>/<reino>/<personaje>`). El diario está en disco y
/// pudo tocarse a mano: los nombres tienen que ser normales, ningún tramo que exista puede ser un enlace y todos los
/// cambios tienen que caer dentro de la carpeta.
fn carpeta_del_personaje(original: &Diario) -> Result<(PathBuf, String), ErrorNucleo> {
    let destino = &original.destino;
    let base = format!(
        "{CARPETA_WTF}/{CARPETA_ACCOUNT}/{}/{}/{}",
        destino.cuenta, destino.carpeta_reino, destino.carpeta_personaje
    );
    let fuera = || ErrorNucleo::escritura(Path::new(&base), &"ruta fuera de WTF/Account");
    let carpeta = ruta_segura(&destino.raiz_version, &base).ok_or_else(fuera)?;
    let prefijo = format!("{base}/");
    if !original
        .cambios
        .iter()
        .all(|c| c.ruta.starts_with(&prefijo))
    {
        return Err(fuera());
    }
    let mut tramo = destino.raiz_version.clone();
    for parte in Path::new(&base).components() {
        tramo.push(parte);
        if es_enlace(&tramo) {
            return Err(enlace(&tramo));
        }
    }
    Ok((carpeta, base))
}

/// Los ficheros que hay ahora en la carpeta del personaje (si el juego la volvió a crear), por su ruta relativa en
/// minúsculas: Windows no distingue mayúsculas. `ficheros_de` rechaza los enlaces de dentro.
fn ficheros_actuales(
    carpeta: &Path,
    base: &str,
) -> Result<BTreeMap<String, (String, PathBuf)>, ErrorNucleo> {
    if !carpeta.exists() {
        return Ok(BTreeMap::new());
    }
    Ok(ficheros_de(carpeta)?
        .into_iter()
        .map(|fichero| {
            let ruta = format!("{base}/{}", relativa_con_barras(carpeta, &fichero));
            (ruta.to_lowercase(), (ruta, fichero))
        })
        .collect())
}

/// Un `restaurar` por cada fichero de la original que hoy no está como antes, y un `borrar` por cada fichero que hay
/// ahora y antes no estaba. Los que ya están como antes no se tocan.
fn construir(
    original_dir: &Path,
    original: Diario,
    mut actuales: BTreeMap<String, (String, PathBuf)>,
) -> Result<Preparado, ErrorNucleo> {
    let mut cambios = Vec::new();
    let mut fuentes = Vec::new();
    let mut copias = 0;
    for cambio in &original.cambios {
        let (Some(sha), Some(bytes), Some(copia)) = (
            &cambio.antes.sha256,
            cambio.antes.bytes,
            &cambio.antes.copia,
        ) else {
            return Err(ErrorNucleo::NoDeshacible {});
        };
        actuales.remove(&cambio.ruta.to_lowercase());
        let actual = contenido(&en_disco(&original, cambio)?)?;
        if actual.as_ref().is_some_and(|(hash, _)| hash == sha) {
            continue;
        }
        cambios.push(Cambio {
            ruta: cambio.ruta.clone(),
            accion: Accion::Restaurar,
            de_cuenta: false,
            antes: antes_de(actual, &mut copias),
            despues: Despues {
                existe: true,
                sha256: Some(sha.clone()),
                bytes: Some(bytes),
            },
        });
        fuentes.push(Some(original_dir.join(copia)));
    }
    for (ruta, fichero) in actuales.into_values() {
        cambios.push(Cambio {
            ruta,
            accion: Accion::Borrar,
            de_cuenta: false,
            antes: antes_de(contenido(&fichero)?, &mut copias),
            despues: Despues::NO_EXISTE,
        });
        fuentes.push(None);
    }
    if cambios.len() > MAXIMO_CAMBIOS {
        return Err(ErrorNucleo::escritura(
            original_dir,
            &format!("más de {MAXIMO_CAMBIOS} ficheros"),
        ));
    }
    Ok(Preparado {
        original,
        cambios,
        fuentes,
    })
}

/// `antes` de un cambio del deshacer: lo que hay ahora, con su número de copia, o «no existía».
fn antes_de(actual: Option<(String, u64)>, copias: &mut usize) -> Antes {
    let Some((sha, bytes)) = actual else {
        return Antes::NO_EXISTIA;
    };
    let antes = Antes::guardado(sha, bytes, copia_n(*copias));
    *copias += 1;
    antes
}

/// Paso 5 del deshacer: cada `restaurar` escribe la copia de la original (verificada antes y después) y cada `borrar`
/// quita el fichero. Para en el primer fallo; `ejecutar` restaura.
fn aplicar(diario: &Diario, fuentes: &[Option<PathBuf>]) -> Result<(), ErrorNucleo> {
    for (cambio, fuente) in diario.cambios.iter().zip(fuentes) {
        let destino = en_disco(diario, cambio)?;
        sigue_como_antes(&destino, cambio)?;
        match (cambio.accion, fuente, &cambio.despues.sha256) {
            (Accion::Borrar, _, _) => {
                std::fs::remove_file(&destino).map_err(|e| ErrorNucleo::escritura(&destino, &e))?;
            }
            (Accion::Restaurar, Some(fuente), Some(sha)) => {
                let bytes = comprobar(fuente, sha)?;
                escribir_atomico(&destino, &bytes)?;
                comprobar(&destino, sha)?;
            }
            (Accion::Restaurar, _, _) => {
                return Err(ErrorNucleo::escritura(
                    &destino,
                    &"sin copia de la que restaurar",
                ));
            }
        }
    }
    Ok(())
}

/// SHA-256 de la lista de cambios (ruta y hash de lo que hay ahora): si algo cambia entre el plan y la escritura, cambia.
fn huella(cambios: &[Cambio]) -> String {
    let texto: String = cambios
        .iter()
        .map(|c| format!("{}\0{}\n", c.ruta, c.antes.sha256.as_deref().unwrap_or("-")))
        .collect();
    sha256(texto.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::super::tests::{ADDONS_TXT, Escenario, PRAT_LUA, diario_de, escenario};
    use super::*;
    use std::fs::{create_dir_all, read_to_string, write};

    const RECREADO: &str = "Prat-3.0: enabled\r\nEllesmereUI: enabled\r\n";

    fn eliminado(e: &Escenario) -> String {
        e.operaciones
            .eliminar_personaje(&e.personaje, || false)
            .unwrap()
    }

    fn deshacer_ya(e: &Escenario, id: &str) -> Result<String, ErrorNucleo> {
        let plan = e.operaciones.plan_deshacer(id)?;
        e.operaciones.deshacer(id, &plan.huella, || false)
    }

    fn ficheros_del_personaje(e: &Escenario) -> Vec<(String, String)> {
        let carpeta = e.personaje.carpeta();
        ficheros_de(&carpeta)
            .unwrap()
            .into_iter()
            .map(|f| {
                (
                    relativa_con_barras(&carpeta, &f),
                    read_to_string(&f).unwrap(),
                )
            })
            .collect()
    }

    fn operaciones_en_disco(e: &Escenario) -> usize {
        subcarpetas(&e.datos).unwrap().len()
    }

    #[test]
    fn devuelve_la_carpeta_eliminada_tal_como_estaba() {
        let e = escenario();
        let antes = ficheros_del_personaje(&e);
        let id = eliminado(&e);

        let plan = e.operaciones.plan_deshacer(&id).unwrap();
        assert_eq!((plan.vuelven, plan.sobran), (2, 0));
        assert!(plan.cambian.is_empty(), "{plan:?}");
        let nuevo = e.operaciones.deshacer(&id, &plan.huella, || false).unwrap();

        assert_eq!(ficheros_del_personaje(&e), antes);
        let original = diario_de(&e.datos, &id);
        assert_eq!(original.estado, Estado::Deshecha);
        assert_eq!(original.deshecha_por.as_deref(), Some(nuevo.as_str()));
        let deshacer = diario_de(&e.datos, &nuevo);
        assert_eq!(deshacer.tipo, Tipo::Deshacer);
        assert_eq!(deshacer.estado, Estado::Completa);
        assert_eq!(deshacer.deshace.as_deref(), Some(id.as_str()));
    }

    #[test]
    fn el_diario_del_deshacer_tiene_la_forma_del_schema() {
        let e = escenario();
        let id = eliminado(&e);
        let nuevo = deshacer_ya(&e, &id).unwrap();
        let bytes = std::fs::read(e.datos.join(&nuevo).join("operacion.json")).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["tipo"], "deshacer");
        assert_eq!(json["deshace"], id.as_str());
        assert_eq!(json["cambios"][0]["accion"], "restaurar");
        assert_eq!(
            json["cambios"][0]["antes"],
            serde_json::json!({ "existia": false })
        );
        assert_eq!(json["cambios"][0]["despues"]["existe"], true);
        assert_eq!(
            json["cambios"][0]["despues"]["bytes"],
            ADDONS_TXT.len() as u64
        );
        let original = std::fs::read(e.datos.join(&id).join("operacion.json")).unwrap();
        let original: serde_json::Value = serde_json::from_slice(&original).unwrap();
        assert_eq!(original["estado"], "deshecha");
        assert_eq!(original["deshechaPor"], nuevo.as_str());
    }

    /// El juego volvió a crear la carpeta: un fichero distinto, uno igual al de antes y uno que antes no estaba.
    #[test]
    fn con_la_carpeta_recreada_avisa_guarda_lo_que_pisa_y_quita_lo_que_sobra() {
        let e = escenario();
        let antes = ficheros_del_personaje(&e);
        let id = eliminado(&e);
        let carpeta = e.personaje.carpeta();
        create_dir_all(carpeta.join("SavedVariables")).unwrap();
        write(carpeta.join("AddOns.txt"), RECREADO).unwrap();
        write(carpeta.join("SavedVariables/Prat-3.0.lua"), PRAT_LUA).unwrap();
        write(carpeta.join("SavedVariables/Nuevo.lua"), "x").unwrap();

        let plan = e.operaciones.plan_deshacer(&id).unwrap();
        let base = e.personaje.ruta_relativa();
        assert_eq!(
            plan.cambian,
            [
                format!("{base}/AddOns.txt"),
                format!("{base}/SavedVariables/Nuevo.lua")
            ]
        );
        let nuevo = e.operaciones.deshacer(&id, &plan.huella, || false).unwrap();

        assert_eq!(ficheros_del_personaje(&e), antes);
        let deshacer = diario_de(&e.datos, &nuevo);
        let pisado = &deshacer.cambios[0];
        let copia = e
            .datos
            .join(&nuevo)
            .join(pisado.antes.copia.as_ref().unwrap());
        assert_eq!(read_to_string(copia).unwrap(), RECREADO);
        assert_eq!(deshacer.cambios[1].accion, Accion::Borrar);
    }

    #[test]
    fn si_algo_cambia_tras_el_plan_no_escribe_nada() {
        let e = escenario();
        let id = eliminado(&e);
        let plan = e.operaciones.plan_deshacer(&id).unwrap();
        create_dir_all(e.personaje.carpeta()).unwrap();
        write(e.personaje.carpeta().join("AddOns.txt"), RECREADO).unwrap();

        assert_eq!(
            e.operaciones.deshacer(&id, &plan.huella, || false),
            Err(ErrorNucleo::PlanCaducado {})
        );
        assert_eq!(
            read_to_string(e.personaje.carpeta().join("AddOns.txt")).unwrap(),
            RECREADO
        );
        assert_eq!(operaciones_en_disco(&e), 1);
        assert_eq!(diario_de(&e.datos, &id).estado, Estado::Completa);
    }

    #[test]
    fn con_el_juego_abierto_no_deshace() {
        let e = escenario();
        let id = eliminado(&e);
        let plan = e.operaciones.plan_deshacer(&id).unwrap();

        assert_eq!(
            e.operaciones.deshacer(&id, &plan.huella, || true),
            Err(ErrorNucleo::JuegoAbierto {})
        );
        assert!(!e.personaje.carpeta().exists());
        assert_eq!(operaciones_en_disco(&e), 1);
    }

    #[test]
    fn solo_se_deshace_una_eliminacion_completa() {
        let e = escenario();
        let id = eliminado(&e);
        let nuevo = deshacer_ya(&e, &id).unwrap();

        assert_eq!(deshacer_ya(&e, &id), Err(ErrorNucleo::NoDeshacible {}));
        assert_eq!(deshacer_ya(&e, &nuevo), Err(ErrorNucleo::NoDeshacible {}));
        assert_eq!(
            deshacer_ya(&e, "../otra"),
            Err(ErrorNucleo::OperacionNoEncontrada {})
        );
    }

    #[test]
    fn con_otro_juego_en_la_raiz_no_deshace() {
        let e = escenario();
        let id = eliminado(&e);
        let raiz = &e.personaje.raiz_version;
        write(
            raiz.join(".flavor.info"),
            "Product Flavor!STRING:0\r\nwow\r\n",
        )
        .unwrap();

        assert!(matches!(
            e.operaciones.plan_deshacer(&id),
            Err(ErrorNucleo::JuegoDistinto { .. })
        ));
        std::fs::rename(raiz, raiz.with_extension("movida")).unwrap();
        assert!(matches!(
            e.operaciones.plan_deshacer(&id),
            Err(ErrorNucleo::JuegoDistinto { .. })
        ));
    }

    #[test]
    fn una_posterior_que_toca_los_mismos_ficheros_va_primero() {
        let e = escenario();
        let primera = eliminado(&e);
        // Se fecha en el pasado: las dos caen en el mismo segundo.
        let mut diario = diario_de(&e.datos, &primera);
        diario.creada = "2000-01-01T00:00:00Z".into();
        guardar(&e.datos.join(&primera), &diario).unwrap();
        create_dir_all(e.personaje.carpeta()).unwrap();
        write(e.personaje.carpeta().join("AddOns.txt"), RECREADO).unwrap();
        let segunda = eliminado(&e);

        assert_eq!(
            e.operaciones.plan_deshacer(&primera),
            Err(ErrorNucleo::OperacionPosterior {
                id: segunda.clone()
            })
        );
        deshacer_ya(&e, &segunda).unwrap();
        // La segunda ya está deshecha: ahora la primera puede ir, y pisa lo que devolvió la segunda.
        let plan = e.operaciones.plan_deshacer(&primera).unwrap();
        assert_eq!(plan.cambian.len(), 1);
    }

    /// Un diario tocado a mano no saca el deshacer de la carpeta del personaje.
    #[test]
    fn una_ruta_del_diario_fuera_del_personaje_no_se_usa() {
        let e = escenario();
        let id = eliminado(&e);
        let dir = e.datos.join(&id);
        let mut diario = diario_de(&e.datos, &id);
        diario.cambios[0].ruta = "WTF/Account/1#1/93/Goldberg-Joestar/AddOns.txt".into();
        guardar(&dir, &diario).unwrap();

        assert!(matches!(
            e.operaciones.plan_deshacer(&id),
            Err(ErrorNucleo::EscrituraFallida { .. })
        ));
    }

    #[test]
    fn al_arrancar_marca_deshecha_la_original_si_se_corto_entre_los_dos_pasos() {
        let e = escenario();
        let id = eliminado(&e);
        let nuevo = deshacer_ya(&e, &id).unwrap();
        let mut original = diario_de(&e.datos, &id);
        original.estado = Estado::Completa;
        original.deshecha_por = None;
        guardar(&e.datos.join(&id), &original).unwrap();

        e.operaciones.recuperar(|| false);

        let original = diario_de(&e.datos, &id);
        assert_eq!(original.estado, Estado::Deshecha);
        assert_eq!(original.deshecha_por, Some(nuevo));
    }

    /// Un fichero recreado que otro programa tiene abierto sin dejar sustituirlo: el deshacer devuelve lo que ya había
    /// escrito y no pisa nada.
    #[test]
    #[cfg(windows)]
    fn si_falla_a_mitad_deja_la_carpeta_como_la_encontro() {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 1;

        let e = escenario();
        let id = eliminado(&e);
        let carpeta = e.personaje.carpeta();
        create_dir_all(carpeta.join("SavedVariables")).unwrap();
        let prat = carpeta.join("SavedVariables/Prat-3.0.lua");
        write(&prat, "recreado").unwrap();
        let plan = e.operaciones.plan_deshacer(&id).unwrap();
        let abierto = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&prat)
            .unwrap();

        let resultado = e.operaciones.deshacer(&id, &plan.huella, || false);

        drop(abierto);
        assert!(
            matches!(
                resultado,
                Err(ErrorNucleo::OperacionFallida {
                    restaurada: true,
                    ..
                })
            ),
            "{resultado:?}"
        );
        assert!(
            !carpeta.join("AddOns.txt").exists(),
            "se quita lo ya escrito"
        );
        assert_eq!(read_to_string(&prat).unwrap(), "recreado");
        assert_eq!(diario_de(&e.datos, &id).estado, Estado::Completa);
    }

    /// Una unión de Windows donde estaba la carpeta del personaje: no se sigue.
    #[test]
    #[cfg(windows)]
    fn una_union_en_el_camino_se_rechaza() {
        let e = escenario();
        let id = eliminado(&e);
        let fuera = e._tmp.path().join("fuera");
        create_dir_all(&fuera).unwrap();
        let creada = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(e.personaje.carpeta())
            .arg(&fuera)
            .output()
            .unwrap();
        assert!(creada.status.success(), "{creada:?}");

        let resultado = e.operaciones.plan_deshacer(&id);

        assert!(
            matches!(resultado, Err(ErrorNucleo::EnlaceNoAdmitido { .. })),
            "{resultado:?}"
        );
        assert!(std::fs::read_dir(&fuera).unwrap().next().is_none());
        std::fs::remove_dir(e.personaje.carpeta()).unwrap();
    }
}
