//! Importación (`docs/arquitectura.md`, specs 024 y 025): del paquete leído y los personajes destino al plan que enseña
//! la vista previa, y de ahí a las operaciones que lo escriben. Planificar es puro: no escribe en ningún sitio. Leer los
//! ficheros actuales va aparte (`leer_destino`) y escribir es de Operaciones (`aplicar.rs`).

mod aplicar;
mod contenido;
mod elementos;
mod fusion;

use crate::deteccion::{self, PersonajeUbicado};
use crate::error::{ErrorNucleo, MotivoPaquete};
use crate::operaciones::{Operaciones, PaqueteImportado, sha256};
use crate::paquetes::{
    Ambito, Categoria, EntradaLeida, ManifiestoLeido, PaqueteLeido, Tipo, leer_fichero,
};
use crate::piezas::{Filtro, Fuente, Quitadas, fuente_de_destino};
pub use aplicar::ResultadoPersonaje;
pub use contenido::Contenido;
pub use elementos::Elemento;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

const PREFIJO_CUENTA: &str = "cuenta/";

/// Lo que se enseña al abrir el paquete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resumen {
    pub sha256: String,
    pub manifiesto: ManifiestoLeido,
    pub entradas: Vec<EntradaResumen>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntradaResumen {
    pub id: String,
    pub categoria: String,
    pub ambito: Ambito,
    /// Se escribe en la carpeta de la cuenta: afecta a todos sus personajes y hay que decidirla.
    pub de_cuenta: bool,
    pub bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incompatible: Option<MotivoIncompatible>,
    /// Qué trae, para enseñarlo. Solo en las entradas que se importan y cuya categoría tiene lista.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contenido: Option<Contenido>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MotivoIncompatible {
    /// Categoría o tipo que esta versión aún no importa (configuración de addons, Modo Edición, uno de una 1.x más
    /// nueva).
    NoSeImporta,
    /// El `destino` no es el fichero de su categoría (spec 024, decisión 7).
    DestinoAjeno,
}

/// Lo que elige el jugador.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pedido {
    /// Ids de las entradas marcadas.
    pub seleccion: Vec<String>,
    /// Qué hacer con cada entrada de cuenta en cada cuenta afectada. Sin decisión, es un conflicto.
    pub cuentas: Vec<DecisionCuenta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionCuenta {
    pub cuenta: String,
    pub entrada: String,
    pub aceptar: bool,
}

/// Un personaje destino con lo que tiene ahora en disco: por `destino` lógico, el contenido o `None` si no existe.
#[derive(Debug, Clone)]
pub struct Destino {
    pub personaje: PersonajeUbicado,
    pub actuales: HashMap<String, Option<Vec<u8>>>,
    /// Carpetas de los addons instalados, en minúsculas.
    pub instalados: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub personajes: Vec<PlanPersonaje>,
    pub incompatibles: Vec<Incompatible>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Incompatible {
    pub entrada: String,
    pub categoria: String,
    pub motivo: MotivoIncompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanPersonaje {
    pub cuenta: String,
    pub reino: String,
    pub personaje: String,
    pub ficheros: Vec<FicheroPlan>,
    /// Entradas de cuenta sin decidir en la cuenta de este personaje.
    pub conflictos: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Grupo {
    Nuevo,
    Modificado,
    SinCambios,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Accion {
    Crear,
    Sobrescribir,
    Fusionar,
}

/// Un fichero del plan. Lo que no se serializa es para la entrega 2: dónde se escribe, qué se escribe y la huella.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FicheroPlan {
    pub entrada: String,
    pub categoria: Categoria,
    /// Ruta lógica (`personaje/…` o `cuenta/…`).
    pub destino: String,
    pub de_cuenta: bool,
    pub grupo: Grupo,
    pub accion: Accion,
    /// Macros, teclas, opciones y addons: qué cambia dentro. El resto: vacío (se sustituye el fichero).
    pub elementos: Vec<Elemento>,
    /// Opciones del paquete que no se importan (lista negra) o addons activados que el destino no tiene, por motivo.
    pub apartadas: Vec<Quitadas>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_antes: Option<u64>,
    pub bytes_despues: u64,
    /// Entrada de cuenta que ya escribe el plan de otro personaje de la misma cuenta (su carpeta).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escrita_en: Option<String>,
    #[serde(skip)]
    pub ruta: PathBuf,
    /// El fichero, relativo a la carpeta del personaje o de su cuenta (de la tabla de piezas).
    #[serde(skip)]
    pub fichero: &'static str,
    #[serde(skip)]
    pub contenido: Vec<u8>,
    /// Huella: SHA-256 del fichero tal como estaba al planificar, o `None` si no existía.
    #[serde(skip)]
    pub antes_sha256: Option<String>,
}

/// Una entrada que se puede importar, con la fuente que dice a qué fichero va.
struct Importable<'a> {
    entrada: &'a EntradaLeida,
    categoria: Categoria,
    fuente: &'static Fuente,
}

/// Una entrada se importa si esta versión conoce su categoría y su tipo, y su `destino` es exactamente el de su
/// categoría. Nada del `destino` que trae el paquete llega a una ruta: la ruta sale de la tabla.
fn clasificar(entrada: &EntradaLeida) -> Result<Importable<'_>, MotivoIncompatible> {
    let tipo_admitido = matches!(
        entrada.tipo_conocido(),
        Some(Tipo::Fichero | Tipo::FicheroFiltrado)
    );
    let categoria = entrada
        .categoria_conocida()
        .filter(|_| tipo_admitido)
        .ok_or(MotivoIncompatible::NoSeImporta)?;
    let fuente =
        fuente_de_destino(categoria, &entrada.destino).ok_or(MotivoIncompatible::DestinoAjeno)?;
    Ok(Importable {
        entrada,
        categoria,
        fuente,
    })
}

fn es_de_cuenta(destino: &str) -> bool {
    destino.starts_with(PREFIJO_CUENTA)
}

pub fn resumen(paquete: &PaqueteLeido) -> Resumen {
    let entradas = paquete
        .manifiesto
        .entradas
        .iter()
        .map(|e| {
            let clasificada = clasificar(e);
            let que_trae = clasificada.as_ref().ok().and_then(|importable| {
                paquete
                    .pieza(&e.id)
                    .and_then(|pieza| contenido::contenido(importable.categoria, pieza))
            });
            EntradaResumen {
                id: e.id.clone(),
                categoria: e.categoria.clone(),
                ambito: e.ambito,
                de_cuenta: es_de_cuenta(&e.destino),
                bytes: e.bytes,
                incompatible: clasificada.err(),
                contenido: que_trae,
            }
        })
        .collect();
    Resumen {
        sha256: paquete.sha256.clone(),
        manifiesto: paquete.manifiesto.clone(),
        entradas,
    }
}

/// Todos de una misma instalación y de la versión del juego del paquete, sin repetir. Si no, no se planifica.
pub fn comprobar_destinos(
    personajes: &[PersonajeUbicado],
    producto: &str,
) -> Result<(), ErrorNucleo> {
    let Some(primero) = personajes.first() else {
        return Err(ErrorNucleo::DestinosNoValidos {});
    };
    let mut vistos = HashSet::new();
    let validos = personajes.iter().all(|p| {
        p.raiz_version == primero.raiz_version
            && p.producto.codigo() == producto
            && vistos.insert(p.carpeta())
    });
    if validos {
        Ok(())
    } else {
        Err(ErrorNucleo::DestinosNoValidos {})
    }
}

/// Lee del disco lo que el paquete podría tocar en este personaje y sus addons instalados. Con un enlace o una unión
/// de Windows en el camino no se sigue: podría salir del `WTF` (como en Operaciones).
pub fn leer_destino(
    personaje: PersonajeUbicado,
    paquete: &PaqueteLeido,
) -> Result<Destino, ErrorNucleo> {
    personaje.comprobar_sin_enlaces()?;
    let mut actuales = HashMap::new();
    for importable in paquete
        .manifiesto
        .entradas
        .iter()
        .filter_map(|e| clasificar(e).ok())
    {
        let ruta = ruta_de(&personaje, importable.fuente);
        actuales.insert(importable.fuente.destino.to_owned(), leer_si_existe(&ruta)?);
    }
    let instalados = deteccion::addons_de(&personaje.raiz_version, personaje.producto)?
        .addons
        .iter()
        .map(|a| a.carpeta.to_lowercase())
        .collect();
    Ok(Destino {
        personaje,
        actuales,
        instalados,
    })
}

/// El contenido de un fichero, o `None` solo si no existe. Si no se puede mirar, es un error: un fichero que existe y no
/// se ve no puede planificarse como «crear» (aplicar lo pisaría sin copia).
fn leer_si_existe(ruta: &Path) -> Result<Option<Vec<u8>>, ErrorNucleo> {
    let datos = match std::fs::symlink_metadata(ruta) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ErrorNucleo::lectura(ruta, &e)),
        Ok(datos) => datos,
    };
    if datos.file_type().is_symlink() {
        return Err(ErrorNucleo::EnlaceNoAdmitido {
            ruta: ruta.display().to_string(),
        });
    }
    if !datos.is_file() {
        return Err(ErrorNucleo::lectura(
            ruta,
            &std::io::Error::other("no es un fichero"),
        ));
    }
    std::fs::read(ruta)
        .map(Some)
        .map_err(|e| ErrorNucleo::lectura(ruta, &e))
}

/// La ruta real de una pieza: su fichero (de la tabla) en la carpeta del personaje o en la de su cuenta.
fn ruta_de(personaje: &PersonajeUbicado, fuente: &Fuente) -> PathBuf {
    let carpeta = if es_de_cuenta(fuente.destino) {
        personaje.carpeta_cuenta()
    } else {
        personaje.carpeta()
    };
    carpeta.join(fuente.fichero)
}

/// El plan de importar `paquete` en `destinos` con lo que pide el jugador. Puro. Una entrada sin su pieza es un paquete
/// roto (leerlo ya lo impide): nunca se planifica como un fichero vacío.
pub fn planificar(
    paquete: &PaqueteLeido,
    destinos: &[Destino],
    pedido: &Pedido,
) -> Result<Plan, ErrorNucleo> {
    let mut incompatibles = Vec::new();
    let mut importables = Vec::new();
    for entrada in &paquete.manifiesto.entradas {
        if !pedido.seleccion.contains(&entrada.id) {
            continue;
        }
        match clasificar(entrada) {
            Ok(importable) => {
                let pieza =
                    paquete
                        .pieza(&entrada.id)
                        .ok_or_else(|| ErrorNucleo::PaqueteInvalido {
                            motivo: MotivoPaquete::PiezaIlegible,
                            detalle: entrada.id.clone(),
                        })?;
                importables.push((importable, pieza));
            }
            Err(motivo) => incompatibles.push(Incompatible {
                entrada: entrada.id.clone(),
                categoria: entrada.categoria.clone(),
                motivo,
            }),
        }
    }
    // Cuenta → carpeta del primer personaje que la escribe.
    let mut cuentas_escritas: HashMap<&str, &str> = HashMap::new();
    let personajes = destinos
        .iter()
        .map(|destino| planificar_personaje(destino, &importables, pedido, &mut cuentas_escritas))
        .collect();
    Ok(Plan {
        personajes,
        incompatibles,
    })
}

fn planificar_personaje<'a>(
    destino: &'a Destino,
    importables: &[(Importable<'_>, &[u8])],
    pedido: &Pedido,
    cuentas_escritas: &mut HashMap<&'a str, &'a str>,
) -> PlanPersonaje {
    let personaje = &destino.personaje;
    let mut ficheros = Vec::new();
    let mut conflictos = Vec::new();
    let mut escribe_cuenta = false;
    for (importable, pieza) in importables {
        let de_cuenta = es_de_cuenta(importable.fuente.destino);
        let mut escrita_en = None;
        if de_cuenta {
            match decision(pedido, &personaje.cuenta, &importable.entrada.id) {
                None => {
                    conflictos.push(importable.entrada.id.clone());
                    continue;
                }
                Some(false) => continue,
                Some(true) => {}
            }
            escrita_en = cuentas_escritas
                .get(personaje.cuenta.as_str())
                .map(|carpeta| (*carpeta).to_owned());
            escribe_cuenta |= escrita_en.is_none();
        }
        let mut fichero = planificar_fichero(destino, importable, pieza);
        if escrita_en.is_some() {
            fichero.grupo = Grupo::SinCambios;
            fichero.escrita_en = escrita_en;
        }
        ficheros.push(fichero);
    }
    if escribe_cuenta {
        cuentas_escritas.insert(&personaje.cuenta, &personaje.personaje);
    }
    PlanPersonaje {
        cuenta: personaje.cuenta.clone(),
        reino: personaje.reino.clone(),
        personaje: personaje.personaje.clone(),
        ficheros,
        conflictos,
    }
}

fn decision(pedido: &Pedido, cuenta: &str, entrada: &str) -> Option<bool> {
    pedido
        .cuentas
        .iter()
        .find(|d| d.cuenta == cuenta && d.entrada == entrada)
        .map(|d| d.aceptar)
}

/// Qué queda en un fichero: con fusión (opciones, addons) o sustituido por la pieza (el resto).
fn planificar_fichero(destino: &Destino, importable: &Importable<'_>, pieza: &[u8]) -> FicheroPlan {
    let fuente = importable.fuente;
    let antes = destino.actuales.get(fuente.destino).cloned().flatten();
    let actual = antes.as_deref().unwrap_or_default();
    let (contenido, elementos, apartadas) = match fuente.filtro {
        Filtro::Opciones => partes(fusion::fusionar_opciones(actual, pieza)),
        Filtro::Addons => partes(fusion::fusionar_addons(actual, pieza, &destino.instalados)),
        Filtro::Ninguno => {
            let elementos = match importable.categoria {
                Categoria::Macros => elementos::diferencias_macros(actual, pieza),
                Categoria::Teclas => elementos::diferencias_teclas(actual, pieza),
                _ => Vec::new(),
            };
            (pieza.to_vec(), elementos, Vec::new())
        }
    };
    let grupo = match &antes {
        None => Grupo::Nuevo,
        Some(a) if *a == contenido => Grupo::SinCambios,
        Some(_) => Grupo::Modificado,
    };
    let accion = match (&antes, fuente.filtro) {
        (None, _) => Accion::Crear,
        (Some(_), Filtro::Ninguno) => Accion::Sobrescribir,
        (Some(_), _) => Accion::Fusionar,
    };
    FicheroPlan {
        entrada: importable.entrada.id.clone(),
        categoria: importable.categoria,
        destino: fuente.destino.to_owned(),
        de_cuenta: es_de_cuenta(fuente.destino),
        grupo,
        accion,
        elementos,
        apartadas,
        bytes_antes: antes.as_ref().map(|a| a.len() as u64),
        bytes_despues: contenido.len() as u64,
        escrita_en: None,
        ruta: ruta_de(&destino.personaje, fuente),
        fichero: fuente.fichero,
        antes_sha256: antes.as_deref().map(sha256),
        contenido,
    }
}

fn partes(fusion: fusion::Fusion) -> (Vec<u8>, Vec<Elemento>, Vec<Quitadas>) {
    (fusion.contenido, fusion.cambios, fusion.apartadas)
}

/// Lo que enseña la vista previa: el plan y lo que hay que avisar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VistaPrevia {
    pub plan: Plan,
    /// Con WoW abierto se puede mirar, pero aplicar no podrá.
    pub juego_abierto: bool,
    /// Número de interfaz de la instalación destino, si se sabe: si no es el del paquete, se avisa.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interfaz_destino: Option<u32>,
    /// Lo que hay que devolver al aplicar: si el plan ya no es este, no se escribe nada.
    pub huella: String,
}

/// El paquete abierto en la vista «Importar» (spec 024, decisión 10): la interfaz lo nombra por su SHA-256, nunca por
/// su ruta. Solo hay uno; abrir otro o cerrar la vista lo olvida. La ruta se guarda para volver a leerlo al aplicar.
#[derive(Debug, Default)]
pub struct PaquetesAbiertos(Mutex<Option<(PathBuf, Arc<PaqueteLeido>)>>);

impl PaquetesAbiertos {
    /// Lee y comprueba el fichero entero y, si es válido, lo deja abierto. Si no, el anterior se olvida igual.
    pub fn abrir(&self, ruta: &Path) -> Result<Resumen, ErrorNucleo> {
        let leido = leer_fichero(ruta);
        let mut abierto = self.cerrojo();
        *abierto = None;
        let paquete = leido?;
        let resumen = resumen(&paquete);
        *abierto = Some((ruta.to_path_buf(), Arc::new(paquete)));
        Ok(resumen)
    }

    pub fn cerrar(&self) {
        *self.cerrojo() = None;
    }

    /// El plan de importar el paquete abierto en esos personajes. Solo lee.
    pub fn vista_previa(
        &self,
        sha256: &str,
        personajes: Vec<PersonajeUbicado>,
        pedido: &Pedido,
        juego_abierto: bool,
    ) -> Result<VistaPrevia, ErrorNucleo> {
        let paquete = self.abierto(sha256)?;
        comprobar_destinos(&personajes, &paquete.manifiesto.origen.producto)?;
        let interfaz_destino = match personajes.first() {
            Some(p) => deteccion::origen_de(&p.raiz_version, p.producto)?.1,
            None => None,
        };
        let destinos = personajes
            .into_iter()
            .map(|p| leer_destino(p, &paquete))
            .collect::<Result<Vec<_>, _>>()?;
        let plan = planificar(&paquete, &destinos, pedido)?;
        Ok(VistaPrevia {
            huella: aplicar::huella(&plan),
            plan,
            juego_abierto,
            interfaz_destino,
        })
    }

    /// Aplica el paquete abierto en esos personajes (spec 025): lo vuelve a leer del disco (tiene que seguir siendo el
    /// mismo), rehace el plan y solo escribe si sale la huella que se enseñó. Una operación por personaje.
    pub fn aplicar(
        &self,
        sha256: &str,
        huella_vista: &str,
        personajes: Vec<PersonajeUbicado>,
        pedido: &Pedido,
        operaciones: &Operaciones,
        juego_en_marcha: impl Fn() -> bool + Copy,
    ) -> Result<Vec<ResultadoPersonaje>, ErrorNucleo> {
        let (ruta, _) = self.abierto_con_ruta(sha256)?;
        let paquete = leer_fichero(&ruta)
            .ok()
            .filter(|p| p.sha256 == sha256)
            .ok_or(ErrorNucleo::PaqueteCambiado {})?;
        comprobar_destinos(&personajes, &paquete.manifiesto.origen.producto)?;
        let destinos = personajes
            .into_iter()
            .map(|p| leer_destino(p, &paquete))
            .collect::<Result<Vec<_>, _>>()?;
        let plan = planificar(&paquete, &destinos, pedido)?;
        if aplicar::huella(&plan) != huella_vista {
            return Err(ErrorNucleo::PlanCaducado {});
        }
        let importado = PaqueteImportado {
            nombre: paquete.manifiesto.perfil.nombre.clone(),
            sha256: paquete.sha256.clone(),
        };
        aplicar::aplicar(&plan, &destinos, &importado, operaciones, juego_en_marcha)
    }

    fn abierto(&self, sha256: &str) -> Result<Arc<PaqueteLeido>, ErrorNucleo> {
        self.abierto_con_ruta(sha256).map(|(_, paquete)| paquete)
    }

    fn abierto_con_ruta(&self, sha256: &str) -> Result<(PathBuf, Arc<PaqueteLeido>), ErrorNucleo> {
        self.cerrojo()
            .as_ref()
            .filter(|(_, p)| p.sha256 == sha256)
            .cloned()
            .ok_or(ErrorNucleo::PaqueteNoAbierto {})
    }

    /// Un hilo que falló con el cerrojo cogido no deja nada a medias: solo se cambia el `Option` entero.
    fn cerrojo(&self) -> MutexGuard<'_, Option<(PathBuf, Arc<PaqueteLeido>)>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deteccion::Producto;
    use serde_json::{Value, json};
    use std::fs::{create_dir_all, write};
    use std::path::Path;

    const MACROS_PJ: &[u8] =
        b"VER 3 0100000000000001 \"Foco\" \"1\"\r\n/cast [@focus] Maleficio\r\nEND\r\n";
    const MACROS_DESTINO: &[u8] = b"VER 3 0100000000000001 \"Viejo\" \"1\"\r\n/say hola\r\nEND\r\n";
    const MACROS_CUENTA: &[u8] = b"VER 3 0000000000000002 \"RS\" \"1\"\r\n/rs\r\nEND\r\n";
    const VENTANAS: &[u8] = b"layout\r\n";

    fn entrada(id: &str, categoria: &str, tipo: &str, destino: &str) -> Value {
        let mut entrada = json!({
            "id": id, "categoria": categoria, "tipo": tipo,
            "ambito": if destino.starts_with("cuenta/") { "cuenta" } else { "personaje" },
            "destino": destino, "sha256": "0".repeat(64), "bytes": 0
        });
        if tipo == "fichero-filtrado" {
            entrada["claves_quitadas"] = json!([]);
        }
        if categoria == "config-addon" {
            entrada["addon"] = json!("Dominos");
        }
        entrada
    }

    fn paquete() -> PaqueteLeido {
        let manifiesto = json!({
            "formato": { "nombre": "wowprofile", "version": "1.0" },
            "perfil": { "nombre": "Druida", "creado": "2026-11-18T10:00:00Z" },
            "origen": { "producto": "wowf", "build": "1.60.1.70938", "interfaz": 16001 },
            "addons": [],
            "entradas": [
                entrada("macros", "macros", "fichero", "personaje/macros-cache.txt"),
                entrada("macros-cuenta", "macros", "fichero", "cuenta/macros-cache.txt"),
                entrada("chat", "chat", "fichero", "personaje/chat-cache.txt"),
                entrada("ventanas", "ventanas", "fichero", "personaje/layout-local.txt"),
                entrada("opciones", "opciones", "fichero-filtrado", "personaje/config-cache.wtf"),
                entrada("dominos", "config-addon", "fichero", "personaje/SavedVariables/Dominos.lua"),
                entrada("colada", "macros", "fichero", "personaje/config-cache.wtf"),
            ]
        });
        PaqueteLeido::de_prueba(
            serde_json::from_value(manifiesto).unwrap(),
            &[
                ("macros", MACROS_PJ),
                ("macros-cuenta", MACROS_CUENTA),
                ("chat", b"WINDOW 1\r\n"),
                ("ventanas", VENTANAS),
                (
                    "opciones",
                    b"SET a \"2\"\r\nSET AllowDangerousScripts \"1\"\r\n",
                ),
                ("dominos", b"x"),
                ("colada", b"SET AllowDangerousScripts \"1\"\r\n"),
            ],
        )
    }

    fn personaje(raiz: &Path, cuenta: &str, carpeta: &str) -> PersonajeUbicado {
        PersonajeUbicado {
            raiz_version: raiz.join("_forever_"),
            producto: Producto::Forever,
            cuenta: cuenta.into(),
            reino: "93".into(),
            personaje: carpeta.into(),
            nombre_en_datos: None,
        }
    }

    /// Un destino en memoria: lo que «hay» en disco se pasa tal cual.
    fn destino(cuenta: &str, carpeta: &str, actuales: &[(&str, &[u8])]) -> Destino {
        Destino {
            personaje: personaje(Path::new("C:/wow"), cuenta, carpeta),
            actuales: actuales
                .iter()
                .map(|(d, b)| ((*d).to_owned(), Some(b.to_vec())))
                .collect(),
            instalados: HashSet::new(),
        }
    }

    fn pedido(seleccion: &[&str], cuentas: &[(&str, &str, bool)]) -> Pedido {
        Pedido {
            seleccion: seleccion.iter().map(|s| (*s).to_owned()).collect(),
            cuentas: cuentas
                .iter()
                .map(|(cuenta, entrada, aceptar)| DecisionCuenta {
                    cuenta: (*cuenta).to_owned(),
                    entrada: (*entrada).to_owned(),
                    aceptar: *aceptar,
                })
                .collect(),
        }
    }

    fn planificar_ok(paquete: &PaqueteLeido, destinos: &[Destino], pedido: &Pedido) -> Plan {
        planificar(paquete, destinos, pedido).unwrap()
    }

    fn fichero<'a>(plan: &'a PlanPersonaje, entrada: &str) -> &'a FicheroPlan {
        plan.ficheros.iter().find(|f| f.entrada == entrada).unwrap()
    }

    #[test]
    fn cada_entrada_cae_en_su_grupo_con_su_detalle_y_su_huella() {
        let destinos = [destino(
            "1#1",
            "Goldberg",
            &[
                ("personaje/macros-cache.txt", MACROS_DESTINO),
                ("personaje/layout-local.txt", VENTANAS),
                (
                    "personaje/config-cache.wtf",
                    b"SET a \"1\"\r\nSET b \"3\"\r\n",
                ),
            ],
        )];
        let todas = [
            "macros", "chat", "ventanas", "opciones", "dominos", "colada",
        ];

        let plan = planificar_ok(&paquete(), &destinos, &pedido(&todas, &[]));

        let pj = &plan.personajes[0];
        let macros = fichero(pj, "macros");
        assert_eq!(
            (macros.grupo, macros.accion),
            (Grupo::Modificado, Accion::Sobrescribir)
        );
        assert_eq!(macros.contenido, MACROS_PJ);
        assert_eq!(macros.elementos[0].antes.as_deref(), Some("Viejo"));
        assert_eq!(macros.antes_sha256, Some(sha256(MACROS_DESTINO)));
        assert!(
            macros
                .ruta
                .ends_with("Account/1#1/93/Goldberg/macros-cache.txt")
        );
        let chat = fichero(pj, "chat");
        assert_eq!(
            (chat.grupo, chat.accion, chat.antes_sha256.as_deref()),
            (Grupo::Nuevo, Accion::Crear, None)
        );
        assert_eq!(fichero(pj, "ventanas").grupo, Grupo::SinCambios);
        let opciones = fichero(pj, "opciones");
        assert_eq!(
            (opciones.grupo, opciones.accion),
            (Grupo::Modificado, Accion::Fusionar)
        );
        assert_eq!(opciones.contenido, b"SET a \"2\"\r\nSET b \"3\"\r\n");
        assert_eq!(
            crate::piezas::todas_las_claves(&opciones.apartadas),
            ["AllowDangerousScripts"]
        );
        let incompatibles: Vec<_> = plan
            .incompatibles
            .iter()
            .map(|i| (i.entrada.as_str(), i.motivo))
            .collect();
        assert_eq!(
            incompatibles,
            [
                ("dominos", MotivoIncompatible::NoSeImporta),
                ("colada", MotivoIncompatible::DestinoAjeno)
            ]
        );
        assert_eq!(
            pj.ficheros.len(),
            4,
            "lo incompatible no llega a ningún fichero"
        );
    }

    #[test]
    fn lo_no_seleccionado_no_esta_en_el_plan() {
        let plan = planificar_ok(
            &paquete(),
            &[destino("1#1", "A", &[])],
            &pedido(&["chat"], &[]),
        );
        let entradas: Vec<_> = plan.personajes[0]
            .ficheros
            .iter()
            .map(|f| f.entrada.as_str())
            .collect();
        assert_eq!(entradas, ["chat"]);
        assert!(plan.incompatibles.is_empty());
    }

    #[test]
    fn lo_de_cuenta_se_decide_por_cuenta_y_se_escribe_una_vez() {
        let destinos = [
            destino("1#1", "Primero", &[]),
            destino("1#1", "Segundo", &[]),
            destino("2#1", "OtraCuenta", &[]),
            destino("3#1", "Quitada", &[]),
        ];
        let decisiones = [
            ("1#1", "macros-cuenta", true),
            ("3#1", "macros-cuenta", false),
        ];

        let plan = planificar_ok(
            &paquete(),
            &destinos,
            &pedido(&["macros-cuenta"], &decisiones),
        );

        let primero = fichero(&plan.personajes[0], "macros-cuenta");
        assert_eq!((primero.grupo, primero.de_cuenta), (Grupo::Nuevo, true));
        assert!(primero.ruta.ends_with("Account/1#1/macros-cache.txt"));
        let segundo = fichero(&plan.personajes[1], "macros-cuenta");
        assert_eq!(
            (segundo.grupo, segundo.escrita_en.as_deref()),
            (Grupo::SinCambios, Some("Primero"))
        );
        assert!(plan.personajes[2].ficheros.is_empty());
        assert_eq!(plan.personajes[2].conflictos, ["macros-cuenta"]);
        assert!(plan.personajes[3].ficheros.is_empty() && plan.personajes[3].conflictos.is_empty());
    }

    #[test]
    fn el_resumen_dice_que_es_de_cuenta_y_que_no_se_importa() {
        let resumen = resumen(&paquete());
        let entradas: Vec<_> = resumen
            .entradas
            .iter()
            .map(|e| (e.id.as_str(), e.de_cuenta, e.incompatible))
            .collect();
        assert_eq!(entradas[1], ("macros-cuenta", true, None));
        assert_eq!(entradas[0], ("macros", false, None));
        assert_eq!(
            entradas[5],
            ("dominos", false, Some(MotivoIncompatible::NoSeImporta))
        );
        assert_eq!(
            entradas[6],
            ("colada", false, Some(MotivoIncompatible::DestinoAjeno))
        );
    }

    #[test]
    fn el_resumen_trae_el_contenido_solo_de_lo_que_se_importa() {
        let resumen = resumen(&paquete());
        let contenido = |id: &str| {
            resumen
                .entradas
                .iter()
                .find(|e| e.id == id)
                .and_then(|e| e.contenido.clone())
        };
        assert_eq!(
            contenido("opciones"),
            Some(Contenido::Lista {
                elementos: vec!["a".into()]
            })
        );
        assert_eq!(contenido("ventanas"), None);
        assert_eq!(contenido("chat"), None);
        assert_eq!(contenido("dominos"), None);
        assert_eq!(contenido("colada"), None);
    }

    #[test]
    fn los_destinos_son_de_una_instalacion_y_del_producto_del_paquete() {
        let raiz = Path::new("C:/wow");
        let a = personaje(raiz, "1#1", "A");
        let b = personaje(raiz, "2#1", "B");
        assert_eq!(comprobar_destinos(&[a.clone(), b.clone()], "wowf"), Ok(()));
        let mut otra_raiz = b.clone();
        otra_raiz.raiz_version = Path::new("D:/copia/_forever_").to_path_buf();
        for destinos in [
            vec![],
            vec![a.clone(), a.clone()],
            vec![a.clone(), otra_raiz],
        ] {
            assert_eq!(
                comprobar_destinos(&destinos, "wowf"),
                Err(ErrorNucleo::DestinosNoValidos {})
            );
        }
        assert_eq!(
            comprobar_destinos(&[a], "wow"),
            Err(ErrorNucleo::DestinosNoValidos {})
        );
    }

    /// Todos los ficheros bajo `raiz` con su contenido.
    fn arbol(raiz: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut ficheros = Vec::new();
        let mut pendientes = vec![raiz.to_path_buf()];
        while let Some(carpeta) = pendientes.pop() {
            for entrada in std::fs::read_dir(carpeta).unwrap() {
                let ruta = entrada.unwrap().path();
                if ruta.is_dir() {
                    pendientes.push(ruta);
                } else {
                    let contenido = std::fs::read(&ruta).unwrap();
                    ficheros.push((ruta, contenido));
                }
            }
        }
        ficheros.sort();
        ficheros
    }

    #[test]
    fn una_entrada_sin_su_pieza_no_se_planifica_como_vacia() {
        let mut paquete = paquete();
        let manifiesto = paquete.manifiesto.clone();
        paquete = PaqueteLeido::de_prueba(manifiesto, &[("macros", MACROS_PJ)]);
        let error = planificar(
            &paquete,
            &[destino("1#1", "A", &[])],
            &pedido(&["chat"], &[]),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ErrorNucleo::PaqueteInvalido {
                motivo: MotivoPaquete::PiezaIlegible,
                ..
            }
        ));
    }

    #[test]
    fn un_fichero_que_no_se_puede_mirar_no_se_planifica_como_nuevo() {
        let tmp = tempfile::tempdir().unwrap();
        let pj = personaje(tmp.path(), "1#1", "Goldberg");
        // Una carpeta donde va un fichero: existe, pero no se puede leer como fichero.
        create_dir_all(pj.carpeta().join("macros-cache.txt")).unwrap();
        assert!(matches!(
            leer_destino(pj, &paquete()).unwrap_err(),
            ErrorNucleo::LecturaFallida { .. }
        ));
    }

    #[cfg(windows)]
    #[test]
    fn una_union_en_el_camino_no_se_sigue() {
        let tmp = tempfile::tempdir().unwrap();
        let fuera = tmp.path().join("fuera");
        create_dir_all(&fuera).unwrap();
        write(fuera.join("macros-cache.txt"), b"secreto").unwrap();
        let pj = personaje(tmp.path(), "1#1", "Goldberg");
        create_dir_all(pj.carpeta().parent().unwrap()).unwrap();
        let creada = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(pj.carpeta())
            .arg(&fuera)
            .output()
            .unwrap();
        assert!(creada.status.success(), "{creada:?}");
        assert!(matches!(
            leer_destino(pj, &paquete()).unwrap_err(),
            ErrorNucleo::EnlaceNoAdmitido { .. }
        ));
    }

    #[test]
    fn el_paquete_abierto_se_nombra_por_su_hash_y_un_fallo_olvida_el_anterior() {
        let tmp = tempfile::tempdir().unwrap();
        let bueno = tmp.path().join("bueno.wowprofile");
        let piezas = vec![crate::paquetes::tests::pieza("macros", b"VER 3\r\n")];
        let perfil = crate::paquetes::Perfil {
            nombre: "Druida".into(),
            descripcion: None,
            creado: "2026-11-18T10:00:00Z".into(),
        };
        let origen = crate::paquetes::Origen {
            producto: "wowf",
            build: "1.60.1.70938".into(),
            interfaz: 16001,
        };
        crate::paquetes::Paquete::nuevo(perfil, origen, Vec::new(), piezas)
            .unwrap()
            .guardar(&bueno)
            .unwrap();
        let malo = tmp.path().join("malo.wowprofile");
        write(&malo, b"no es un zip").unwrap();
        let abiertos = PaquetesAbiertos::default();

        let resumen = abiertos.abrir(&bueno).unwrap();
        assert!(abiertos.abierto(&resumen.sha256).is_ok());
        assert_eq!(
            abiertos.abierto(&"0".repeat(64)).unwrap_err(),
            ErrorNucleo::PaqueteNoAbierto {}
        );
        assert!(abiertos.abrir(&malo).is_err());
        assert_eq!(
            abiertos.abierto(&resumen.sha256).unwrap_err(),
            ErrorNucleo::PaqueteNoAbierto {}
        );
        abiertos.abrir(&bueno).unwrap();
        abiertos.cerrar();
        assert_eq!(
            abiertos.abierto(&resumen.sha256).unwrap_err(),
            ErrorNucleo::PaqueteNoAbierto {}
        );
    }

    #[test]
    fn leer_y_planificar_no_escriben_nada() {
        let tmp = tempfile::tempdir().unwrap();
        let pj = personaje(tmp.path(), "1#1", "Goldberg");
        create_dir_all(pj.carpeta()).unwrap();
        write(pj.carpeta().join("macros-cache.txt"), MACROS_DESTINO).unwrap();
        let addons = pj
            .raiz_version
            .join("Interface")
            .join("AddOns")
            .join("Dominos");
        create_dir_all(&addons).unwrap();
        write(addons.join("Dominos.toc"), "## Title: Dominos\r\n").unwrap();
        let antes = arbol(tmp.path());

        let destino = leer_destino(pj, &paquete()).unwrap();
        let plan = planificar_ok(
            &paquete(),
            std::slice::from_ref(&destino),
            &pedido(
                &["macros", "macros-cuenta", "chat"],
                &[("1#1", "macros-cuenta", true)],
            ),
        );

        assert_eq!(arbol(tmp.path()), antes);
        assert_eq!(
            destino.actuales.get("personaje/macros-cache.txt"),
            Some(&Some(MACROS_DESTINO.to_vec()))
        );
        assert_eq!(destino.actuales.get("cuenta/macros-cache.txt"), Some(&None));
        assert!(
            !destino
                .actuales
                .contains_key("personaje/SavedVariables/Dominos.lua")
        );
        assert!(destino.instalados.contains("dominos"));
        assert_eq!(plan.personajes[0].ficheros.len(), 3);
    }
}
