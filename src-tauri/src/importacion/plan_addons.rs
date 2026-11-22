//! El plan de la configuración de addons (#72): cada fichero se escribe una sola vez, con todas
//! las entradas que van a él aplicadas en el orden del paquete. Un fichero de cuenta lleva lo de todos los destinos de
//! esa cuenta y se escribe en la operación del primero; en los demás sale «sin cambios», escrito en el primero.

use super::config_addon::{AccionPerfil, Aplicacion, ConflictoPerfil, EntradaAddon, NoAplicable};
use super::{Accion, AvisoFichero, Destino, FicheroPlan, Grupo, Pedido, decision, ruta_de};
use crate::operaciones::sha256;
use crate::paquetes::Categoria;
use std::collections::HashSet;

/// Lo que la configuración de addons añade al plan de un personaje.
#[derive(Debug, Default)]
pub(super) struct DelPersonaje {
    pub ficheros: Vec<FicheroPlan>,
    pub conflictos: Vec<String>,
    pub conflictos_perfil: Vec<ConflictoPerfil>,
    pub no_aplicables: Vec<NoAplicable>,
}

/// Una entrada `config-addon` elegida, con su pieza.
type ConPieza<'a> = (EntradaAddon<'a>, &'a [u8]);

/// Lo de cada destino, en el orden de `destinos`.
pub(super) fn planificar(
    destinos: &[Destino],
    entradas: &[ConPieza<'_>],
    pedido: &Pedido,
) -> Vec<DelPersonaje> {
    let mut salida: Vec<DelPersonaje> = destinos.iter().map(|_| DelPersonaje::default()).collect();
    let mut ficheros: Vec<(bool, &str)> = Vec::new();
    for (entrada, _) in entradas {
        let fichero = (entrada.en_cuenta, entrada.fichero.as_str());
        if !ficheros.contains(&fichero) {
            ficheros.push(fichero);
        }
    }
    for (en_cuenta, fichero) in ficheros {
        let del_fichero: Vec<&ConPieza<'_>> = entradas
            .iter()
            .filter(|(e, _)| e.en_cuenta == en_cuenta && e.fichero == fichero)
            .collect();
        for grupo in grupos(destinos, en_cuenta) {
            let mut fichero = Fichero {
                destinos,
                indices: &grupo,
                pedido,
                salida: &mut salida,
            };
            fichero.planificar(&del_fichero);
        }
    }
    salida
}

/// Los destinos que comparten el fichero: los de una misma cuenta si es de cuenta, cada uno solo si no.
fn grupos(destinos: &[Destino], en_cuenta: bool) -> Vec<Vec<usize>> {
    if !en_cuenta {
        return (0..destinos.len()).map(|i| vec![i]).collect();
    }
    let mut grupos: Vec<Vec<usize>> = Vec::new();
    for (i, destino) in destinos.iter().enumerate() {
        let cuenta = &destino.personaje.cuenta;
        match grupos
            .iter_mut()
            .find(|g| destinos[g[0]].personaje.cuenta == *cuenta)
        {
            Some(grupo) => grupo.push(i),
            None => grupos.push(vec![i]),
        }
    }
    grupos
}

/// Un fichero real y los destinos que lo comparten (el primero es el que lo escribe).
struct Fichero<'p, 'd> {
    destinos: &'d [Destino],
    indices: &'p [usize],
    pedido: &'p Pedido,
    salida: &'p mut [DelPersonaje],
}

impl Fichero<'_, '_> {
    fn primero(&self) -> &Destino {
        &self.destinos[self.indices[0]]
    }

    fn planificar(&mut self, entradas: &[&ConPieza<'_>]) {
        let Some((muestra, _)) = entradas.first() else {
            return;
        };
        let antes = self
            .primero()
            .actuales
            .get(muestra.entrada.destino.as_str())
            .cloned()
            .flatten();
        let mut contenido = antes.clone().unwrap_or_default();
        let mut aplicadas: Vec<&EntradaAddon<'_>> = Vec::new();
        let mut elementos = vec![Vec::new(); self.indices.len()];
        let mut conflictos_vistos = HashSet::new();
        for (entrada, pieza) in entradas {
            if !self.decidida(entrada) {
                continue;
            }
            let mut aplicada = false;
            for (k, &i) in self.indices.iter().enumerate() {
                let destino = &self.destinos[i];
                let resultado =
                    config_addon_aplicar(entrada, pieza, &contenido, destino, self.pedido);
                match resultado {
                    Aplicacion::Hecha {
                        contenido: nuevo,
                        elementos: cambios,
                    } => {
                        contenido = nuevo;
                        elementos[k].extend(cambios);
                        aplicada = true;
                    }
                    Aplicacion::Conflicto(conflicto) => {
                        if conflictos_vistos.insert(conflicto.entrada.clone()) {
                            self.salida[i].conflictos_perfil.push(conflicto);
                        }
                    }
                    Aplicacion::NoAplicable(motivo) => {
                        self.salida[i].no_aplicables.push(NoAplicable {
                            entrada: entrada.entrada.id.clone(),
                            motivo,
                        })
                    }
                }
            }
            if aplicada {
                aplicadas.push(entrada);
            }
        }
        if aplicadas.is_empty() {
            return;
        }
        self.repartir(&aplicadas, antes, contenido, elementos);
    }

    /// Una entrada que afecta a toda la cuenta solo se aplica si se aceptó en esa cuenta. Sin decidir, es un conflicto
    /// en cada destino de la cuenta.
    fn decidida(&mut self, entrada: &EntradaAddon<'_>) -> bool {
        if !entrada.afecta_cuenta() {
            return true;
        }
        let cuenta = &self.primero().personaje.cuenta;
        match decision(self.pedido, cuenta, &entrada.entrada.id) {
            Some(aceptar) => aceptar,
            None => {
                for &i in self.indices {
                    self.salida[i].conflictos.push(entrada.entrada.id.clone());
                }
                false
            }
        }
    }

    /// El fichero en el plan del primero, con todo lo que cambia; en los demás, «sin cambios» y escrito en el primero.
    fn repartir(
        &mut self,
        aplicadas: &[&EntradaAddon<'_>],
        antes: Option<Vec<u8>>,
        contenido: Vec<u8>,
        elementos: Vec<Vec<super::Elemento>>,
    ) {
        let primero = self.primero();
        let muestra = aplicadas[0];
        let grupo = match &antes {
            None => Grupo::Nuevo,
            Some(a) if *a == contenido => Grupo::SinCambios,
            Some(_) => Grupo::Modificado,
        };
        let sustituye = !muestra.en_cuenta && matches!(muestra.forma, super::Forma::Generico);
        let accion = match (&antes, sustituye) {
            (None, _) => Accion::Crear,
            (Some(_), true) => Accion::Sobrescribir,
            (Some(_), false) => Accion::Fusionar,
        };
        let plan = FicheroPlan {
            entrada: muestra.entrada.id.clone(),
            entradas: aplicadas.iter().map(|e| e.entrada.id.clone()).collect(),
            categoria: Categoria::ConfigAddon,
            destino: muestra.entrada.destino.clone(),
            de_cuenta: aplicadas.iter().any(|e| e.afecta_cuenta()),
            en_cuenta: muestra.en_cuenta,
            grupo,
            accion,
            elementos: elementos.concat(),
            apartadas: Vec::new(),
            avisos: avisos(primero, aplicadas),
            bytes_antes: antes.as_ref().map(|a| a.len() as u64),
            bytes_despues: contenido.len() as u64,
            escrita_en: None,
            ruta: ruta_de(&primero.personaje, muestra.en_cuenta, &muestra.fichero),
            fichero: muestra.fichero.clone(),
            antes_sha256: antes.as_deref().map(sha256),
            contenido,
        };
        let escritor = primero.personaje.personaje.clone();
        for (k, &i) in self.indices.iter().enumerate().skip(1) {
            self.salida[i].ficheros.push(FicheroPlan {
                grupo: Grupo::SinCambios,
                elementos: elementos[k].clone(),
                escrita_en: Some(escritor.clone()),
                ..plan.clone()
            });
        }
        self.salida[self.indices[0]].ficheros.push(plan);
    }
}

fn config_addon_aplicar(
    entrada: &EntradaAddon<'_>,
    pieza: &[u8],
    contenido: &[u8],
    destino: &Destino,
    pedido: &Pedido,
) -> Aplicacion {
    super::config_addon::aplicar(
        entrada,
        pieza,
        contenido,
        destino.personaje.nombre_en_datos.as_deref(),
        decision_perfil(pedido, &destino.personaje.cuenta, &entrada.entrada.id),
    )
}

fn decision_perfil(pedido: &Pedido, cuenta: &str, entrada: &str) -> Option<AccionPerfil> {
    pedido
        .perfiles
        .iter()
        .find(|d| d.cuenta == cuenta && d.entrada == entrada)
        .map(|d| d.accion)
}

/// «Falta el addon» y «versión no medida», una vez por addon (#72, decisión 9).
fn avisos(destino: &Destino, aplicadas: &[&EntradaAddon<'_>]) -> Vec<AvisoFichero> {
    let mut avisos = Vec::new();
    for entrada in aplicadas {
        let carpeta = entrada.addon.to_lowercase();
        let aviso = match destino.versiones.get(&carpeta) {
            None => Some(AvisoFichero::FaltaAddon {
                addon: entrada.addon.to_owned(),
            }),
            Some(instalada) => entrada
                .version_no_medida(instalada.as_deref())
                .map(|medidas| AvisoFichero::VersionNoMedida {
                    addon: entrada.addon.to_owned(),
                    medidas,
                    instalada: instalada.clone(),
                }),
        };
        if let Some(aviso) = aviso
            && !avisos.contains(&aviso)
        {
            avisos.push(aviso);
        }
    }
    avisos
}

#[cfg(test)]
mod tests;
