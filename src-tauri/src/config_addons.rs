//! Configuración de addons para exportar (spec 027, entrega 2 de #56): qué viaja de cada addon instalado, decidido
//! pieza a pieza con el orden de `docs/adaptadores.md` («Orden de decisión»). Solo lee.

use crate::adaptadores::{self, Adaptador, ClavePersonaje, Regla as ReglaAdaptador};
use crate::addons::Addon;
use crate::descubrimiento::CARPETA_SAVED_VARIABLES as CARPETA_SV;
use crate::deteccion::PersonajeUbicado;
use crate::lua::{
    self, CLAVE_NAMESPACES, CLAVE_PERFILES, CLAVE_PERSONAJES, Documento, MARCADOR_PERSONAJE,
    Seleccion, Variable, a_latin1, a_utf8, cadena, campo, campos, clave_entre_corchetes, es_acedb,
    tabla,
};
use crate::paquetes::{
    Ambito, Categoria, DeAddon, MARCADOR, MAX_BYTES_PIEZA, Marcador, Pieza, RefAdaptador, Tipo,
    id_de_addon_valido,
};
use crate::piezas::{Motivo, Quitadas};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const EXTENSION_LUA: &str = ".lua";
/// `perfil` en el schema 1.0.
const MAX_PERFIL: usize = 128;
const MAX_ID: usize = 64;
const PREFIJO_ID: &str = "addon-";
/// GUID de jugador: `Player-<servidor>-<8 cifras hexadecimales>`.
const PREFIJO_GUID: &[u8] = b"Player-";
const CIFRAS_GUID: usize = 8;
/// Los nombres de personaje de WoW tienen de 2 a 12 letras.
const LETRAS_NOMBRE: std::ops::RangeInclusive<usize> = 2..=12;

/// Qué regla de `docs/adaptadores.md` sacó la pieza. Decide si va marcada y qué aviso lleva.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Regla {
    Adaptador,
    PerfilAcedb,
    GenericoCuenta,
    GenericoPersonaje,
    GenericoCuentaConPersonajes,
}

impl Regla {
    pub fn es_generica(self) -> bool {
        !matches!(self, Self::Adaptador | Self::PerfilAcedb)
    }
}

/// Lo que se ha reconocido en una pieza genérica: solo cambia su aviso, nunca la marca.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Pista {
    /// `Player-<servidor>-<id>`.
    Guid,
    /// Claves `Nombre-Reino` o `Nombre - Reino`.
    NombreReino,
}

/// Una pieza `config-addon` con lo que el diálogo necesita para enseñarla.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiezaAddon {
    pub pieza: Pieza,
    pub regla: Regla,
    pub marcada: bool,
    /// El adaptador se midió con otra versión del addon.
    pub version_no_medida: bool,
    pub pistas: Vec<Pista>,
    /// Nombre del perfil AceDB tal como lo ve quien exporta (el manifiesto puede llevar el marcador).
    pub perfil: Option<String>,
}

/// Por qué algo de un addon no se puede exportar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MotivoNoExportable {
    /// No se puede leer el fichero o no se entiende como Lua.
    NoSeLee,
    /// El personaje no tiene nombre en los datos: no se sabe qué perfil usa.
    SinNombre,
    /// El personaje no tiene perfil asignado en ese addon.
    SinPerfil,
    /// El nombre del perfil no es UTF-8 o pasa de lo que admite el formato.
    PerfilNoValido,
    /// La carpeta del addon no cabe en el `destino` del formato.
    CarpetaNoAdmitida,
    /// Pasa del límite del formato por pieza.
    DemasiadoGrande,
    /// Ya trae `@@WOWPROFILE:`.
    TraeMarcador,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoExportable {
    pub addon: String,
    pub ambito: Ambito,
    pub variable: Option<String>,
    pub motivo: MotivoNoExportable,
}

#[derive(Debug, Default)]
pub struct ConfigAddons {
    pub piezas: Vec<PiezaAddon>,
    pub no_exportables: Vec<NoExportable>,
}

/// La configuración de los addons instalados del personaje. `claves_cuenta`: cómo se llaman en los datos los
/// personajes de su cuenta (regla 5).
pub fn de_personaje(
    personaje: &PersonajeUbicado,
    addons: &[Addon],
    claves_cuenta: &HashSet<String>,
) -> ConfigAddons {
    let mut contexto = Contexto {
        cuenta: personaje.carpeta_cuenta().join(CARPETA_SV),
        personaje_sv: personaje.carpeta().join(CARPETA_SV),
        clave: personaje.nombre_en_datos.as_deref(),
        claves_cuenta,
        ids: HashSet::new(),
        salida: ConfigAddons::default(),
    };
    for addon in addons {
        if !destino_valido(&addon.carpeta) {
            contexto.registrar(
                addon,
                Ambito::Cuenta,
                None,
                MotivoNoExportable::CarpetaNoAdmitida,
            );
            continue;
        }
        match adaptadores::de_addon(&addon.carpeta) {
            Some(adaptador) if adaptador.no_exportar => {}
            Some(adaptador) => contexto.por_adaptador(addon, adaptador),
            None => contexto.por_reglas_genericas(addon),
        }
    }
    contexto.salida
}

/// `<carpeta>.lua` tiene que caber en el `destino` del schema: tramos no vacíos entre puntos (ni `..`).
fn destino_valido(carpeta: &str) -> bool {
    id_de_addon_valido(carpeta) && carpeta.split('.').all(|tramo| !tramo.is_empty())
}

struct Contexto<'a> {
    cuenta: PathBuf,
    personaje_sv: PathBuf,
    /// Clave del personaje en los datos (`"Nombre Apellido"`), si se conoce.
    clave: Option<&'a str>,
    claves_cuenta: &'a HashSet<String>,
    ids: HashSet<String>,
    salida: ConfigAddons,
}

/// Una pieza recién sacada, antes de darle id.
struct Nueva {
    ambito: Ambito,
    tipo: Tipo,
    /// Lo que distingue su id de las demás piezas del addon.
    sufijo: String,
    contenido: Vec<u8>,
    quitadas: Vec<Quitadas>,
    de_addon: DeAddon,
    regla: Regla,
    /// La versión del addon es la medida (o no hace falta).
    medida: bool,
    pistas: Vec<Pista>,
    /// Nombre del perfil AceDB tal como lo ve quien exporta.
    perfil: Option<String>,
}

/// Un fichero leído y entendido.
struct Leido {
    bytes: Vec<u8>,
    documento: Documento,
}

impl Contexto<'_> {
    fn carpeta(&self, ambito: Ambito) -> &Path {
        match ambito {
            Ambito::Personaje => &self.personaje_sv,
            Ambito::Cuenta => &self.cuenta,
        }
    }

    /// El fichero del addon en ese ámbito. `None` si no existe o no se puede usar (y entonces se dice por qué).
    fn leer(&mut self, addon: &Addon, ambito: Ambito) -> Option<Leido> {
        let ruta = self
            .carpeta(ambito)
            .join(format!("{}{EXTENSION_LUA}", addon.carpeta));
        if !ruta.is_file() {
            return None;
        }
        let bytes = match std::fs::read(&ruta) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.registrar(addon, ambito, None, MotivoNoExportable::NoSeLee);
                return None;
            }
        };
        if contiene(&bytes, MARCADOR) {
            self.registrar(addon, ambito, None, MotivoNoExportable::TraeMarcador);
            return None;
        }
        let documento = Documento::leer(&bytes);
        if documento.is_none() {
            self.registrar(addon, ambito, None, MotivoNoExportable::NoSeLee);
        }
        documento.map(|documento| Leido { bytes, documento })
    }

    /// Apunta lo que no se puede exportar y por qué.
    fn registrar(
        &mut self,
        addon: &Addon,
        ambito: Ambito,
        variable: Option<&str>,
        motivo: MotivoNoExportable,
    ) {
        self.salida.no_exportables.push(NoExportable {
            addon: addon.carpeta.clone(),
            ambito,
            variable: variable.map(str::to_owned),
            motivo,
        });
    }

    /// Regla 1: solo lo que nombran las reglas del adaptador.
    fn por_adaptador(&mut self, addon: &Addon, adaptador: &Adaptador) {
        let medida = adaptador.version_medida(addon.datos.version.as_deref());
        for ambito in [Ambito::Cuenta, Ambito::Personaje] {
            let reglas: Vec<_> = adaptador
                .reglas
                .iter()
                .filter(|r| r.ambito == ambito)
                .collect();
            if reglas.is_empty() {
                continue;
            }
            let Some(leido) = self.leer(addon, ambito) else {
                continue;
            };
            for regla in reglas {
                let Some(variable) = leido.documento.variable(&regla.variable) else {
                    continue;
                };
                if regla.perfil_acedb {
                    self.perfil_acedb(addon, &leido.documento, &variable, Regla::Adaptador);
                } else {
                    self.regla_de_adaptador(
                        addon,
                        adaptador,
                        regla,
                        &leido.documento,
                        &variable,
                        medida,
                    );
                }
            }
        }
    }

    fn regla_de_adaptador(
        &mut self,
        addon: &Addon,
        adaptador: &Adaptador,
        regla: &ReglaAdaptador,
        documento: &Documento,
        variable: &Variable<'_>,
        medida: bool,
    ) {
        let personaje = match (regla.clave_personaje, self.clave) {
            (ClavePersonaje::NombreApellido, Some(clave)) => a_latin1(clave),
            _ if !regla.solo_personaje.is_empty() => {
                self.registrar(
                    addon,
                    regla.ambito,
                    Some(&regla.variable),
                    MotivoNoExportable::SinNombre,
                );
                return;
            }
            _ => String::new(),
        };
        let seleccion = Seleccion {
            incluir: if regla.entera {
                None
            } else {
                regla.incluir.as_deref()
            },
            excluir: &regla.excluir,
            solo_personaje: &regla.solo_personaje,
            personaje: &personaje,
        };
        let Some(valor) = documento.recortar(variable.valor, &seleccion) else {
            return;
        };
        let mut contenido = format!("{} = ", variable.nombre).into_bytes();
        contenido.extend(valor);
        contenido.extend(documento.fin_de_linea().as_bytes());
        let marcadores = if contiene(&contenido, MARCADOR_PERSONAJE.as_bytes()) {
            vec![Marcador::Personaje]
        } else {
            Vec::new()
        };
        let de_addon = DeAddon {
            addon: addon.carpeta.clone(),
            variable: Some(variable.nombre.clone()),
            perfil: None,
            adaptador: Some(RefAdaptador {
                id: adaptador.id.clone(),
                version: adaptador.version,
            }),
            marcadores,
        };
        self.anadir(
            addon,
            Nueva {
                ambito: regla.ambito,
                tipo: Tipo::Adaptador,
                sufijo: variable.nombre.clone(),
                contenido,
                quitadas: Vec::new(),
                de_addon,
                regla: Regla::Adaptador,
                medida,
                pistas: Vec::new(),
                perfil: None,
            },
        );
    }

    /// Reglas 2–5: AceDB en las variables de cuenta, y el resto de cada fichero desmarcado.
    fn por_reglas_genericas(&mut self, addon: &Addon) {
        if let Some(leido) = self.leer(addon, Ambito::Cuenta) {
            self.cuenta_generica(addon, &leido);
        }
        if let Some(leido) = self.leer(addon, Ambito::Personaje) {
            let pistas = pistas(
                &leido.bytes,
                leido.documento.variables().iter().map(|v| v.valor),
            );
            self.anadir(
                addon,
                Nueva {
                    ambito: Ambito::Personaje,
                    tipo: Tipo::Fichero,
                    sufijo: "personaje".to_owned(),
                    contenido: leido.bytes,
                    quitadas: Vec::new(),
                    de_addon: de_addon(addon),
                    regla: Regla::GenericoPersonaje,
                    medida: true,
                    pistas,
                    perfil: None,
                },
            );
        }
    }

    fn cuenta_generica(&mut self, addon: &Addon, leido: &Leido) {
        let variables = leido.documento.variables();
        let (acedb, resto): (Vec<_>, Vec<_>) = variables.iter().partition(|v| es_acedb(v.valor));
        for variable in &acedb {
            self.perfil_acedb(addon, &leido.documento, variable, Regla::PerfilAcedb);
        }
        if resto.is_empty() {
            return;
        }
        let (tipo, contenido, quitadas) = if acedb.is_empty() {
            (Tipo::Fichero, leido.bytes.clone(), Vec::new())
        } else {
            let fin = leido.documento.fin_de_linea().as_bytes();
            let mut contenido = Vec::new();
            for variable in &resto {
                contenido.extend(leido.documento.bytes_de_variable(variable));
                contenido.extend(fin);
            }
            let quitadas = vec![Quitadas {
                motivo: Motivo::ViajaComoPerfil,
                claves: acedb.iter().map(|v| v.nombre.clone()).collect(),
            }];
            (Tipo::FicheroFiltrado, contenido, quitadas)
        };
        let valores = || resto.iter().map(|v| v.valor);
        let menciona = valores()
            .flat_map(lua::claves_de_cadena)
            .any(|c| self.claves_cuenta.contains(&c));
        let regla = if menciona {
            Regla::GenericoCuentaConPersonajes
        } else {
            Regla::GenericoCuenta
        };
        let pistas = pistas(&contenido, valores());
        self.anadir(
            addon,
            Nueva {
                ambito: Ambito::Cuenta,
                tipo,
                sufijo: "cuenta".to_owned(),
                contenido,
                quitadas,
                de_addon: de_addon(addon),
                regla,
                medida: true,
                pistas,
                perfil: None,
            },
        );
    }

    /// Regla 2: el perfil que usa el personaje, con el de cada namespace (spec 027, decisión 2).
    fn perfil_acedb(
        &mut self,
        addon: &Addon,
        documento: &Documento,
        variable: &Variable<'_>,
        regla: Regla,
    ) {
        let Some(clave) = self.clave else {
            self.registrar(
                addon,
                Ambito::Cuenta,
                Some(&variable.nombre),
                MotivoNoExportable::SinNombre,
            );
            return;
        };
        let Some((nombre, contenido)) = extraer_perfil(documento, variable.valor, &a_latin1(clave))
        else {
            self.registrar(
                addon,
                Ambito::Cuenta,
                Some(&variable.nombre),
                MotivoNoExportable::SinPerfil,
            );
            return;
        };
        let visible = a_utf8(&nombre).filter(|n| (1..=MAX_PERFIL).contains(&n.chars().count()));
        let Some(visible) = visible else {
            self.registrar(
                addon,
                Ambito::Cuenta,
                Some(&variable.nombre),
                MotivoNoExportable::PerfilNoValido,
            );
            return;
        };
        let (perfil, marcadores) = if visible == clave {
            (MARCADOR_PERSONAJE.to_owned(), vec![Marcador::Personaje])
        } else {
            (visible.clone(), Vec::new())
        };
        let de_addon = DeAddon {
            addon: addon.carpeta.clone(),
            variable: Some(variable.nombre.clone()),
            perfil: Some(perfil),
            adaptador: None,
            marcadores,
        };
        self.anadir(
            addon,
            Nueva {
                ambito: Ambito::Cuenta,
                tipo: Tipo::PerfilAcedb,
                sufijo: format!("perfil-{}", variable.nombre),
                contenido,
                quitadas: Vec::new(),
                de_addon,
                regla,
                medida: true,
                pistas: Vec::new(),
                perfil: Some(visible),
            },
        );
    }

    fn anadir(&mut self, addon: &Addon, nueva: Nueva) {
        let Nueva {
            ambito,
            tipo,
            sufijo,
            contenido,
            quitadas,
            de_addon,
            regla,
            medida,
            pistas,
            perfil,
        } = nueva;
        if contenido.len() > MAX_BYTES_PIEZA {
            self.registrar(
                addon,
                ambito,
                de_addon.variable.as_deref(),
                MotivoNoExportable::DemasiadoGrande,
            );
            return;
        }
        let id = id_unico(&mut self.ids, &addon.carpeta, &sufijo);
        let carpeta = match ambito {
            Ambito::Personaje => "personaje",
            Ambito::Cuenta => "cuenta",
        };
        let pieza = Pieza {
            id,
            categoria: Categoria::ConfigAddon,
            tipo,
            ambito,
            destino: format!("{carpeta}/{CARPETA_SV}/{}{EXTENSION_LUA}", addon.carpeta),
            contenido,
            quitadas,
            addon: Some(de_addon),
        };
        self.salida.piezas.push(PiezaAddon {
            pieza,
            regla,
            marcada: !regla.es_generica() && medida,
            version_no_medida: regla == Regla::Adaptador && !medida,
            pistas,
            perfil,
        });
    }
}

fn de_addon(addon: &Addon) -> DeAddon {
    DeAddon {
        addon: addon.carpeta.clone(),
        ..DeAddon::default()
    }
}

/// El nombre del perfil (Latin-1) y la pieza
/// `{ ["profile"] = <perfil>, ["namespaces"] = { ["<módulo>"] = <perfil del módulo> } }`, o `None` si el personaje no
/// tiene perfil asignado. Nunca lleva `char`, `realm` ni `global`.
fn extraer_perfil(
    documento: &Documento,
    valor: &full_moon::ast::Expression,
    clave_latin1: &str,
) -> Option<(String, Vec<u8>)> {
    let raiz = tabla(valor)?;
    let personajes = tabla(campo(raiz, CLAVE_PERSONAJES)?)?;
    let nombre = campos(personajes)
        .find(|(k, _)| k == clave_latin1)
        .and_then(|(_, v)| cadena(v))?;
    let perfil = campo(tabla(campo(raiz, CLAVE_PERFILES)?)?, &nombre)?;
    let fin = documento.fin_de_linea().as_bytes();
    let mut contenido = b"{".to_vec();
    contenido.extend(fin);
    contenido.extend(b"[\"profile\"] = ");
    contenido.extend(documento.bytes_de(perfil));
    contenido.extend(b",");
    contenido.extend(fin);
    let modulos: Vec<_> = campo(raiz, CLAVE_NAMESPACES)
        .and_then(tabla)
        .into_iter()
        .flat_map(campos)
        .filter_map(|(modulo, v)| {
            let perfiles = tabla(campo(tabla(v)?, CLAVE_PERFILES)?)?;
            Some((modulo, campo(perfiles, &nombre)?))
        })
        .collect();
    if !modulos.is_empty() {
        contenido.extend(b"[\"namespaces\"] = {");
        contenido.extend(fin);
        for (modulo, perfil) in modulos {
            contenido.extend(clave_entre_corchetes(&modulo));
            contenido.extend(b" = ");
            contenido.extend(documento.bytes_de(perfil));
            contenido.extend(b",");
            contenido.extend(fin);
        }
        contenido.extend(b"},");
        contenido.extend(fin);
    }
    contenido.extend(b"}");
    contenido.extend(fin);
    Some((nombre, contenido))
}

/// Pistas de otros jugadores (spec 027, decisión 12).
fn pistas<'a>(
    bytes: &[u8],
    valores: impl Iterator<Item = &'a full_moon::ast::Expression>,
) -> Vec<Pista> {
    let mut pistas = Vec::new();
    if tiene_guid(bytes) {
        pistas.push(Pista::Guid);
    }
    if valores
        .flat_map(lua::claves_de_cadena)
        .any(|c| es_nombre_reino(&c))
    {
        pistas.push(Pista::NombreReino);
    }
    pistas
}

fn tiene_guid(bytes: &[u8]) -> bool {
    bytes
        .windows(PREFIJO_GUID.len())
        .enumerate()
        .filter(|(_, v)| *v == PREFIJO_GUID)
        .any(|(i, _)| guid_desde(&bytes[i + PREFIJO_GUID.len()..]))
}

/// `<cifras>-<8 hexadecimales>` al principio de `resto`.
fn guid_desde(resto: &[u8]) -> bool {
    let servidor = resto.iter().take_while(|b| b.is_ascii_digit()).count();
    let Some(tras_guion) = resto.get(servidor..).and_then(|r| r.strip_prefix(b"-")) else {
        return false;
    };
    servidor > 0
        && tras_guion.len() >= CIFRAS_GUID
        && tras_guion[..CIFRAS_GUID].iter().all(u8::is_ascii_hexdigit)
}

/// `Nombre-Reino` o `Nombre - Reino`: nombre de una palabra con mayúscula y de 2 a 12 letras; reino en mayúscula.
fn es_nombre_reino(clave: &str) -> bool {
    let Some((nombre, reino)) = clave.split_once('-') else {
        return false;
    };
    let (nombre, reino) = (
        nombre.strip_suffix(' ').unwrap_or(nombre),
        reino.strip_prefix(' ').unwrap_or(reino),
    );
    let letras = nombre.chars().count();
    let nombre_ok = LETRAS_NOMBRE.contains(&letras)
        && nombre.chars().all(char::is_alphabetic)
        && nombre.chars().next().is_some_and(char::is_uppercase);
    let reino_ok = reino.chars().next().is_some_and(char::is_uppercase)
        && reino
            .chars()
            .all(|c| c.is_alphabetic() || c == ' ' || c == '\'');
    nombre_ok && reino_ok
}

/// `addon-<carpeta>-<sufijo>` en el alfabeto de los ids del formato, recortado y sin repetir.
fn id_unico(usados: &mut HashSet<String>, carpeta: &str, sufijo: &str) -> String {
    let base = format!("{PREFIJO_ID}{}-{}", slug(carpeta), slug(sufijo));
    let recortar = |texto: &str, largo: usize| {
        texto[..texto.len().min(largo)]
            .trim_end_matches('-')
            .to_owned()
    };
    let mut id = recortar(&base, MAX_ID);
    let mut n = 2;
    while !usados.insert(id.clone()) {
        let cola = format!("-{n}");
        id = format!("{}{cola}", recortar(&base, MAX_ID - cola.len()));
        n += 1;
    }
    id
}

/// Minúsculas ASCII; el resto, guiones (sin repetir ni en los bordes).
fn slug(texto: &str) -> String {
    let mut salida = String::new();
    for c in texto.chars() {
        if c.is_ascii_alphanumeric() {
            salida.push(c.to_ascii_lowercase());
        } else if !salida.ends_with('-') {
            salida.push('-');
        }
    }
    salida.trim_matches('-').to_owned()
}

fn contiene(bytes: &[u8], buscado: &[u8]) -> bool {
    bytes.windows(buscado.len()).any(|v| v == buscado)
}

#[cfg(test)]
mod tests;
