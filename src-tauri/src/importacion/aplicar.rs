//! Aplicar el plan (spec 025, entrega 2 de #63): una operación `importacion` por personaje, una detrás de otra. Si una
//! falla, se restaura, las siguientes no se aplican y las anteriores quedan aplicadas.

use super::{Accion, Destino, FicheroPlan, Grupo, Plan, PlanPersonaje};
use crate::error::ErrorNucleo;
use crate::operaciones::{AccionImportar, Escritura, Operaciones, PaqueteImportado, sha256};
use serde::Serialize;

/// Qué pasó en un personaje.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoPersonaje {
    pub cuenta: String,
    pub reino: String,
    pub personaje: String,
    #[serde(flatten)]
    pub resultado: Resultado,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "resultado", rename_all = "camelCase")]
pub enum Resultado {
    /// Se escribió todo: `operacion` es su id en el historial.
    Completa { operacion: String, ficheros: usize },
    /// No había nada que escribir: no se creó operación.
    SinCambios,
    /// Falló. Si `error` es `operacionFallida`, dice si se restauró y en qué operación están las copias.
    Fallida { error: ErrorNucleo },
    /// No se intentó: falló un personaje anterior.
    NoAplicada,
}

/// Los ficheros que de verdad se escriben en un personaje.
fn a_escribir(personaje: &PlanPersonaje) -> impl Iterator<Item = &FicheroPlan> {
    personaje
        .ficheros
        .iter()
        .filter(|f| f.grupo != Grupo::SinCambios)
}

/// Huella del plan: por cada fichero que se escribe, su ruta, cómo estaba y cómo quedará; y lo que sigue sin decidir.
/// Aplicar rehace el plan y solo escribe si sale la misma huella que se enseñó (spec 025, decisión 7).
pub fn huella(plan: &Plan) -> String {
    let mut texto = String::new();
    for personaje in &plan.personajes {
        for fichero in a_escribir(personaje) {
            texto.push_str(&format!(
                "{}\0{}\0{}\n",
                fichero.ruta.display(),
                fichero.antes_sha256.as_deref().unwrap_or("-"),
                sha256(&fichero.contenido)
            ));
        }
        for conflicto in &personaje.conflictos {
            texto.push_str(&format!("?{}/{conflicto}\n", personaje.cuenta));
        }
        for conflicto in &personaje.conflictos_perfil {
            texto.push_str(&format!(
                "?perfil/{}/{}\n",
                personaje.cuenta, conflicto.entrada
            ));
        }
    }
    sha256(texto.as_bytes())
}

/// Aplica `plan` a `destinos` (los mismos con los que se planificó, en el mismo orden). El plan no puede tener nada sin
/// decidir.
pub fn aplicar(
    plan: &Plan,
    destinos: &[Destino],
    paquete: &PaqueteImportado,
    operaciones: &Operaciones,
    juego_en_marcha: impl Fn() -> bool + Copy,
) -> Result<Vec<ResultadoPersonaje>, ErrorNucleo> {
    if plan
        .personajes
        .iter()
        .any(|p| !p.conflictos.is_empty() || !p.conflictos_perfil.is_empty())
    {
        return Err(ErrorNucleo::ConflictosSinDecidir {});
    }
    let mut parar = false;
    let resultados = plan
        .personajes
        .iter()
        .zip(destinos)
        .map(|(plan_personaje, destino)| {
            let resultado = if parar {
                Resultado::NoAplicada
            } else {
                let resultado = aplicar_uno(
                    plan_personaje,
                    destino,
                    paquete,
                    operaciones,
                    juego_en_marcha,
                );
                parar = matches!(resultado, Resultado::Fallida { .. });
                resultado
            };
            ResultadoPersonaje {
                cuenta: plan_personaje.cuenta.clone(),
                reino: plan_personaje.reino.clone(),
                personaje: plan_personaje.personaje.clone(),
                resultado,
            }
        })
        .collect();
    Ok(resultados)
}

fn aplicar_uno(
    plan: &PlanPersonaje,
    destino: &Destino,
    paquete: &PaqueteImportado,
    operaciones: &Operaciones,
    juego_en_marcha: impl Fn() -> bool,
) -> Resultado {
    let escrituras: Vec<Escritura> = a_escribir(plan).map(escritura).collect();
    if escrituras.is_empty() {
        return Resultado::SinCambios;
    }
    let ficheros = escrituras.len();
    match operaciones.importar(
        &destino.personaje,
        paquete.clone(),
        escrituras,
        juego_en_marcha,
    ) {
        Ok(operacion) => Resultado::Completa {
            operacion,
            ficheros,
        },
        Err(error) => Resultado::Fallida { error },
    }
}

fn escritura(fichero: &FicheroPlan) -> Escritura {
    Escritura {
        fichero: fichero.fichero.clone(),
        de_cuenta: fichero.en_cuenta,
        entrada: fichero.entrada.clone(),
        accion: match fichero.accion {
            Accion::Crear => AccionImportar::Crear,
            Accion::Sobrescribir => AccionImportar::Sobrescribir,
            Accion::Fusionar => AccionImportar::Fusionar,
        },
        contenido: fichero.contenido.clone(),
        antes_sha256: fichero.antes_sha256.clone(),
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_modo_edicion;
