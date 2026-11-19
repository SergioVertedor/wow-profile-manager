//! Importar (#63, `docs/flujos.md`, «Aplicar»): escribir en el `WTF` de un personaje los ficheros del plan de
//! importación, con el mismo orden de escritura que las demás operaciones. Una operación por personaje: las entradas de
//! su cuenta van en la del primer personaje de esa cuenta (eso lo decide el plan, no este módulo).
//!
//! Aquí no se planifica nada: se escriben exactamente los bytes del plan, y solo si cada fichero sigue como estaba al
//! planificar. Las carpetas de addons (#73) se instalan después de los ficheros (`carpetas.rs`).

use super::carpetas::{self, Cargar, InstalarCarpeta};
use super::{
    Accion, Antes, Cambio, Despues, Diario, ESQUEMA, Estado, MAXIMO_CAMBIOS, Operaciones, Tipo,
    ahora_rfc3339, base_de, comprobar, comprobar_bases, contenido, copia_n, destino_de, ejecutar,
    en_disco, guardar, sha256, sigue_como_antes,
};
use crate::deteccion::PersonajeUbicado;
use crate::error::ErrorNucleo;
use crate::escritura::escribir_atomico;
use serde::{Deserialize, Serialize};

/// El perfil que se importó, tal como queda en el diario (`paquete` en `docs/operacion.schema.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaqueteImportado {
    pub nombre: String,
    pub sha256: String,
}

/// Cómo se escribe un fichero de la importación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionImportar {
    Crear,
    Sobrescribir,
    Fusionar,
}

impl AccionImportar {
    fn en_diario(self) -> Accion {
        match self {
            Self::Crear => Accion::Crear,
            Self::Sobrescribir => Accion::Sobrescribir,
            Self::Fusionar => Accion::Fusionar,
        }
    }
}

/// Un fichero que escribir, sacado del plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Escritura {
    /// Relativa a la carpeta del personaje o, con `de_cuenta`, a la de su cuenta, con `/`.
    pub fichero: String,
    pub de_cuenta: bool,
    /// El id de la entrada del paquete de la que sale.
    pub entrada: String,
    pub accion: AccionImportar,
    /// Los bytes exactos que quedarán en el fichero.
    pub contenido: Vec<u8>,
    /// El SHA-256 del fichero tal como estaba al planificar, o `None` si no existía.
    pub antes_sha256: Option<String>,
}

impl Operaciones {
    /// Escribe `escrituras` en el `WTF` de `personaje` e instala `carpetas` en `Interface/AddOns` de su versión, como
    /// una operación `importacion`. Si algún fichero ya no está como al planificar o alguna carpeta ya existe, no crea
    /// nada (`PlanCaducado`). Devuelve el id de la operación.
    pub fn importar(
        &self,
        personaje: &PersonajeUbicado,
        paquete: PaqueteImportado,
        escrituras: Vec<Escritura>,
        carpetas: Vec<InstalarCarpeta>,
        cargar: &Cargar,
        juego_en_marcha: impl Fn() -> bool,
    ) -> Result<String, ErrorNucleo> {
        let _turno = self.cerrojo.lock().unwrap_or_else(|e| e.into_inner());
        if juego_en_marcha() {
            return Err(ErrorNucleo::JuegoAbierto {});
        }
        // Como al eliminar: una pendiente se termina antes, para que el siguiente arranque no la restaure encima.
        self.recuperar_sin_cerrojo(&juego_en_marcha);
        self.sin_operacion_por_resolver()?;
        let (mut diario, contenidos) = preparar(personaje, paquete, escrituras)?;
        diario.carpetas = carpetas::preparar(&personaje.raiz_version, carpetas, cargar)?;
        if diario.cambios.is_empty() && diario.carpetas.is_empty() {
            return Err(ErrorNucleo::PlanCaducado {});
        }
        let (id, dir) = self.nueva_carpeta()?;
        diario.id = id;
        if let Err(error) = guardar(&dir, &diario) {
            // Aún no hay copias ni se ha tocado el `WTF`.
            std::fs::remove_dir_all(&dir).ok();
            return Err(error);
        }
        ejecutar(&dir, diario, &juego_en_marcha, |diario| {
            escribir(diario, &contenidos)?;
            carpetas::instalar(
                &diario.destino.raiz_version,
                &diario.id,
                &diario.carpetas,
                cargar,
            )
        })
    }
}

/// Paso 2, todavía sin carpeta: el diario en `preparando`. Primero se comprueba que cada ruta cae en su base (sin
/// enlaces) y después que cada fichero sigue como al planificar. Si algo no cuadra, no se ha creado nada.
fn preparar(
    personaje: &PersonajeUbicado,
    paquete: PaqueteImportado,
    escrituras: Vec<Escritura>,
) -> Result<(Diario, Vec<Vec<u8>>), ErrorNucleo> {
    if escrituras.len() > MAXIMO_CAMBIOS {
        return Err(ErrorNucleo::PlanCaducado {});
    }
    let destino = destino_de(personaje);
    let mut cambios = Vec::with_capacity(escrituras.len());
    let mut planificados = Vec::with_capacity(escrituras.len());
    for escritura in escrituras {
        cambios.push(Cambio {
            ruta: format!(
                "{}/{}",
                base_de(&destino, escritura.de_cuenta),
                escritura.fichero
            ),
            accion: escritura.accion.en_diario(),
            de_cuenta: escritura.de_cuenta,
            entrada: Some(escritura.entrada),
            antes: Antes::NO_EXISTIA,
            despues: Despues {
                existe: true,
                sha256: Some(sha256(&escritura.contenido)),
                bytes: Some(escritura.contenido.len() as u64),
            },
        });
        planificados.push((escritura.contenido, escritura.antes_sha256));
    }
    let mut diario = Diario {
        esquema: ESQUEMA,
        id: String::new(),
        tipo: Tipo::Importacion,
        creada: ahora_rfc3339(),
        terminada: None,
        estado: Estado::Preparando,
        paquete: Some(paquete),
        deshace: None,
        deshecha_por: None,
        error: None,
        destino,
        cambios,
        carpetas: Vec::new(),
    };
    comprobar_bases(&diario)?;
    let mut copias = 0;
    let mut contenidos = Vec::with_capacity(planificados.len());
    for (i, (contenido_nuevo, antes_sha256)) in planificados.into_iter().enumerate() {
        let actual = contenido(&en_disco(&diario, &diario.cambios[i])?)?;
        let cambio = &mut diario.cambios[i];
        cambio.antes = antes_del_plan(actual, antes_sha256.as_deref(), &mut copias)?;
        if (cambio.accion == Accion::Crear) == cambio.antes.existia {
            return Err(ErrorNucleo::PlanCaducado {});
        }
        contenidos.push(contenido_nuevo);
    }
    Ok((diario, contenidos))
}

/// `antes` de un cambio: lo que hay ahora en disco, que tiene que ser lo que vio el plan. Si no, el plan caducó.
fn antes_del_plan(
    actual: Option<(String, u64)>,
    visto: Option<&str>,
    copias: &mut usize,
) -> Result<Antes, ErrorNucleo> {
    match actual {
        None if visto.is_none() => Ok(Antes::NO_EXISTIA),
        Some((sha, bytes)) if visto == Some(sha.as_str()) => {
            let antes = Antes::guardado(sha, bytes, copia_n(*copias));
            *copias += 1;
            Ok(antes)
        }
        _ => Err(ErrorNucleo::PlanCaducado {}),
    }
}

/// Paso 5: cada fichero, si sigue como cuando se copió, recibe los bytes del plan; después se relee contra `despues`.
/// Para en el primer fallo; `ejecutar` restaura.
fn escribir(diario: &Diario, contenidos: &[Vec<u8>]) -> Result<(), ErrorNucleo> {
    for (cambio, bytes) in diario.cambios.iter().zip(contenidos) {
        let ruta = en_disco(diario, cambio)?;
        sigue_como_antes(&ruta, cambio)?;
        escribir_atomico(&ruta, bytes)?;
        if let Some(sha) = &cambio.despues.sha256 {
            comprobar(&ruta, sha)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
