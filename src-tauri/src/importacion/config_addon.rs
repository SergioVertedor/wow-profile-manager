//! La configuración de addons al importar (#72): qué entradas `config-addon` se pueden aplicar y cómo queda el
//! fichero del destino con cada una. Puro: recibe los bytes y devuelve los bytes.
//!
//! Toda pieza tiene que ser solo datos (decisión 1): el juego ejecuta los `SavedVariables` al cargarlos. Un adaptador lo
//! aplica la regla del catálogo de la app, no lo que diga el paquete (decisión 2).

mod perfil;

use super::MotivoIncompatible;
use super::elementos::{Cambio, Clase, Elemento};
use crate::adaptadores::{self, Adaptador, Regla};
use crate::descubrimiento::CARPETA_SAVED_VARIABLES;
use crate::lua::{
    self, CLAVE_PERFIL, Documento, Seleccion, a_utf8, campo, campos, literal_latin1, tabla,
};
use crate::operaciones::segmento_literal;
use crate::paquetes::{Ambito, EntradaLeida, Tipo, id_de_addon_valido};
use full_moon::ast::Expression;
use serde::{Deserialize, Serialize};

const EXTENSION_LUA: &str = ".lua";
const PREFIJO_CUENTA: &str = "cuenta/";
const PREFIJO_PERSONAJE: &str = "personaje/";
/// Palabras reservadas de Lua 5.1: no pueden ser el nombre de una variable.
const RESERVADAS: [&str; 21] = [
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until",
];

/// Una entrada `config-addon` que esta versión sabe aplicar.
#[derive(Debug)]
pub(super) struct EntradaAddon<'a> {
    pub entrada: &'a EntradaLeida,
    pub addon: &'a str,
    /// Va al fichero de la cuenta.
    pub en_cuenta: bool,
    /// Relativo a la carpeta del personaje o de su cuenta: `SavedVariables/<addon>.lua`.
    pub fichero: String,
    pub forma: Forma<'a>,
}

#[derive(Debug)]
pub(super) enum Forma<'a> {
    Perfil {
        variable: &'a str,
        perfil: &'a str,
    },
    Adaptador {
        adaptador: &'static Adaptador,
        regla: &'static Regla,
    },
    Generico,
}

impl EntradaAddon<'_> {
    /// Cambia algo que comparten todos los personajes de la cuenta: se decide en cada cuenta (decisión 6). Un perfil
    /// AceDB solo toca a su personaje; sobrescribir uno que ya existe se decide aparte.
    pub fn afecta_cuenta(&self) -> bool {
        self.en_cuenta && !matches!(self.forma, Forma::Perfil { .. })
    }

    /// Marcada por defecto (decisión 10): lo que sale de un adaptador o de AceDB. Los genéricos, no.
    pub fn marcada(&self) -> bool {
        !matches!(self.forma, Forma::Generico)
    }

    /// El adaptador se midió con otras versiones y su regla deja pasar claves sin nombrar: con esta versión, se avisa.
    pub fn version_no_medida(&self, instalada: Option<&str>) -> Option<Vec<String>> {
        let Forma::Adaptador { adaptador, regla } = self.forma else {
            return None;
        };
        let medida = instalada.is_some_and(|v| adaptador.versiones_addon.iter().any(|m| m == v));
        (regla.deja_pasar_sin_nombrar() && !medida).then(|| adaptador.versiones_addon.clone())
    }
}

/// Si la entrada `config-addon` de tipo `tipo` se puede aplicar, y cómo. `pieza`: sus bytes.
pub(super) fn clasificar<'a>(
    entrada: &'a EntradaLeida,
    tipo: Tipo,
    pieza: &[u8],
) -> Result<EntradaAddon<'a>, MotivoIncompatible> {
    let addon = entrada
        .addon
        .as_deref()
        .filter(|a| id_de_addon_valido(a) && segmento_literal(&format!("{a}{EXTENSION_LUA}")))
        .ok_or(MotivoIncompatible::DestinoAjeno)?;
    let fichero = format!("{CARPETA_SAVED_VARIABLES}/{addon}{EXTENSION_LUA}");
    let en_cuenta = match entrada.destino.strip_suffix(fichero.as_str()) {
        Some(PREFIJO_CUENTA) => true,
        Some(PREFIJO_PERSONAJE) => false,
        _ => return Err(MotivoIncompatible::DestinoAjeno),
    };
    let forma = match tipo {
        Tipo::PerfilAcedb => perfil(entrada, en_cuenta, pieza)?,
        Tipo::Adaptador => de_adaptador(entrada, addon, en_cuenta, pieza)?,
        Tipo::Fichero | Tipo::FicheroFiltrado => {
            let documento = solo_datos(Documento::leer(pieza))?;
            // Un AceDB de cuenta lleva a qué perfil apunta cada personaje: solo entra como `perfil-acedb`.
            if en_cuenta && documento.variables().iter().any(|v| lua::es_acedb(v.valor)) {
                return Err(MotivoIncompatible::PiezaNoValida);
            }
            Forma::Generico
        }
        // Una carpeta de addon es su propia categoría, no configuración.
        Tipo::Carpeta => return Err(MotivoIncompatible::NoSeImporta),
    };
    Ok(EntradaAddon {
        entrada,
        addon,
        en_cuenta,
        fichero,
        forma,
    })
}

fn solo_datos(documento: Option<Documento>) -> Result<Documento, MotivoIncompatible> {
    documento
        .filter(Documento::solo_datos)
        .ok_or(MotivoIncompatible::PiezaNoValida)
}

fn perfil<'a>(
    entrada: &'a EntradaLeida,
    en_cuenta: bool,
    pieza: &[u8],
) -> Result<Forma<'a>, MotivoIncompatible> {
    let variable = entrada
        .variable
        .as_deref()
        .filter(|v| es_identificador(v))
        .ok_or(MotivoIncompatible::PiezaNoValida)?;
    let perfil = entrada
        .perfil
        .as_deref()
        .filter(|p| !p.is_empty())
        .ok_or(MotivoIncompatible::PiezaNoValida)?;
    if !en_cuenta {
        return Err(MotivoIncompatible::DestinoAjeno);
    }
    let documento = solo_datos(Documento::leer_valor(pieza))?;
    let forma_valida = documento
        .valor_suelto()
        .and_then(|v| tabla(v.valor))
        .and_then(|t| campo(t, CLAVE_PERFIL))
        .and_then(tabla)
        .is_some();
    if !forma_valida {
        return Err(MotivoIncompatible::PiezaNoValida);
    }
    Ok(Forma::Perfil { variable, perfil })
}

fn de_adaptador<'a>(
    entrada: &'a EntradaLeida,
    addon: &str,
    en_cuenta: bool,
    pieza: &[u8],
) -> Result<Forma<'a>, MotivoIncompatible> {
    let ambito = if en_cuenta {
        Ambito::Cuenta
    } else {
        Ambito::Personaje
    };
    let referencia = entrada
        .adaptador
        .as_ref()
        .ok_or(MotivoIncompatible::AdaptadorDesconocido)?;
    let adaptador = adaptadores::de_addon(addon)
        .filter(|a| a.id == referencia.id && !a.no_exportar && referencia.version <= a.version)
        .ok_or(MotivoIncompatible::AdaptadorDesconocido)?;
    let regla = adaptador
        .reglas
        .iter()
        .find(|r| {
            r.ambito == ambito
                && !r.perfil_acedb
                && entrada.variable.as_deref() == Some(r.variable.as_str())
        })
        .ok_or(MotivoIncompatible::AdaptadorDesconocido)?;
    let documento = solo_datos(Documento::leer(pieza))?;
    let variables = documento.variables();
    if variables.len() != 1 || variables[0].nombre != regla.variable {
        return Err(MotivoIncompatible::PiezaNoValida);
    }
    Ok(Forma::Adaptador { adaptador, regla })
}

/// Un nombre de variable de Lua.
fn es_identificador(nombre: &str) -> bool {
    let mut letras = nombre.chars();
    letras
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && letras.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !RESERVADAS.contains(&nombre)
}

/// Qué hacer con un perfil que ya existe y es distinto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccionPerfil {
    Renombrar,
    Sobrescribir,
}

/// Un perfil AceDB que ya existe en el destino con otro contenido.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictoPerfil {
    pub entrada: String,
    pub perfil: String,
    /// El nombre que se le pondría al renombrar.
    pub propuesto: String,
    /// Otros personajes del destino que usan ese perfil: sobrescribirlo los cambia también.
    pub usado_por: Vec<String>,
}

/// Por qué una entrada no se aplica en un personaje concreto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MotivoNoAplicable {
    /// El personaje no tiene nombre en los datos: no se sabe con qué clave lo guarda el addon.
    SinNombre,
    /// El fichero del destino no se entiende como Lua: no se pisa lo que no se entiende.
    DestinoIlegible,
    /// La pieza no se entiende, o el fichero que saldría no es solo datos: no se escribe.
    NoEncaja,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoAplicable {
    pub entrada: String,
    pub motivo: MotivoNoAplicable,
}

/// El resultado de aplicar una entrada al contenido de un fichero.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Aplicacion {
    Hecha {
        contenido: Vec<u8>,
        elementos: Vec<Elemento>,
    },
    Conflicto(ConflictoPerfil),
    NoAplicable(MotivoNoAplicable),
}

/// Aplica `entrada` (con su `pieza`) a `actual` (el fichero, o vacío si no existe) para el personaje con clave `clave`
/// (`nombre_en_datos`). `decision`: lo decidido si su perfil ya existe. Si el fichero cambia, lo que sale se vuelve a
/// leer antes de darlo por bueno: si no es solo datos, no se escribe.
pub(super) fn aplicar(
    entrada: &EntradaAddon<'_>,
    pieza: &[u8],
    actual: &[u8],
    clave: Option<&str>,
    decision: Option<AccionPerfil>,
) -> Aplicacion {
    match aplicar_forma(entrada, pieza, actual, clave, decision) {
        Aplicacion::Hecha { contenido, .. }
            if contenido != actual
                && !Documento::leer(&contenido).is_some_and(|d| d.solo_datos()) =>
        {
            Aplicacion::NoAplicable(MotivoNoAplicable::NoEncaja)
        }
        aplicacion => aplicacion,
    }
}

fn aplicar_forma(
    entrada: &EntradaAddon<'_>,
    pieza: &[u8],
    actual: &[u8],
    clave: Option<&str>,
    decision: Option<AccionPerfil>,
) -> Aplicacion {
    if matches!(entrada.forma, Forma::Generico) && !entrada.en_cuenta {
        return generico_de_personaje(actual, pieza);
    }
    let Some(destino) = Documento::leer(actual) else {
        return Aplicacion::NoAplicable(MotivoNoAplicable::DestinoIlegible);
    };
    match entrada.forma {
        Forma::Generico => generico_de_cuenta(&destino, pieza),
        Forma::Adaptador { regla, .. } => por_adaptador(&destino, pieza, regla, clave),
        Forma::Perfil { variable, perfil } => {
            let Some(clave) = clave else {
                return Aplicacion::NoAplicable(MotivoNoAplicable::SinNombre);
            };
            let datos = perfil::DatosPerfil {
                entrada: &entrada.entrada.id,
                variable,
                perfil,
                clave,
            };
            perfil::perfil_acedb(&destino, pieza, &datos, decision)
        }
    }
}

/// Genérico del personaje: la pieza sustituye al fichero entero. Los elementos, las variables que se añaden, cambian o se
/// quitan; si el fichero de antes no se entiende, todas las de la pieza cuentan como añadidas.
fn generico_de_personaje(actual: &[u8], pieza: &[u8]) -> Aplicacion {
    let Some(nuevo) = Documento::leer(pieza) else {
        return Aplicacion::NoAplicable(MotivoNoAplicable::NoEncaja);
    };
    let anterior = Documento::leer(actual);
    let bytes_antes = |nombre: &str| {
        anterior
            .as_ref()
            .and_then(|d| d.variable(nombre).map(|v| d.bytes_de(v.valor)))
    };
    let mut elementos: Vec<Elemento> = nuevo
        .variables()
        .iter()
        .filter_map(|v| {
            let cambio = match bytes_antes(&v.nombre) {
                None => Cambio::Anadido,
                Some(a) if a != nuevo.bytes_de(v.valor) => Cambio::Cambiado,
                Some(_) => return None,
            };
            Some(elemento(&v.nombre, cambio, Clase::Variable))
        })
        .collect();
    let quitadas = anterior
        .iter()
        .flat_map(Documento::variables)
        .filter(|v| nuevo.variable(&v.nombre).is_none())
        .map(|v| elemento(&v.nombre, Cambio::Quitado, Clase::Variable));
    elementos.extend(quitadas);
    Aplicacion::Hecha {
        contenido: pieza.to_vec(),
        elementos,
    }
}

/// Genérico de cuenta: cada variable de la pieza sustituye a la del destino; las demás se quedan. Una variable AceDB del
/// destino no se toca: lleva a qué perfil apunta cada personaje de la cuenta.
fn generico_de_cuenta(destino: &Documento, pieza: &[u8]) -> Aplicacion {
    let Some(pieza) = Documento::leer(pieza) else {
        return Aplicacion::NoAplicable(MotivoNoAplicable::NoEncaja);
    };
    let mut contenido = destino.bytes();
    let mut elementos = Vec::new();
    for variable in pieza.variables() {
        let Some(actual) = Documento::leer(&contenido) else {
            return Aplicacion::NoAplicable(MotivoNoAplicable::DestinoIlegible);
        };
        let nuevo = pieza.bytes_de(variable.valor);
        let destino_actual = actual.variable(&variable.nombre);
        if destino_actual
            .as_ref()
            .is_some_and(|v| lua::es_acedb(v.valor))
        {
            continue;
        }
        let antes = destino_actual.map(|v| actual.bytes_de(v.valor));
        let cambio = match &antes {
            None => Cambio::Anadido,
            Some(a) if *a != nuevo => Cambio::Cambiado,
            Some(_) => continue,
        };
        contenido = actual.con_variable(&variable.nombre, &nuevo);
        elementos.push(elemento(&variable.nombre, cambio, Clase::Variable));
    }
    Aplicacion::Hecha {
        contenido,
        elementos,
    }
}

pub(super) fn elemento(nombre: &str, cambio: Cambio, clase: Clase) -> Elemento {
    Elemento {
        nombre: nombre.to_owned(),
        cambio,
        antes: None,
        despues: None,
        clase: Some(clase),
    }
}

/// Adaptador: la variable fusionada con la regla del catálogo (decisión 2).
fn por_adaptador(
    destino: &Documento,
    pieza: &[u8],
    regla: &Regla,
    clave: Option<&str>,
) -> Aplicacion {
    let personaje = match clave {
        Some(clave) => literal_latin1(clave),
        None if !regla.solo_personaje.is_empty() => {
            return Aplicacion::NoAplicable(MotivoNoAplicable::SinNombre);
        }
        None => String::new(),
    };
    let Some(pieza) = Documento::leer(pieza) else {
        return Aplicacion::NoAplicable(MotivoNoAplicable::NoEncaja);
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
    let Some(contenido) = lua::fusionar_variable(destino, &pieza, &regla.variable, &seleccion)
    else {
        return Aplicacion::Hecha {
            contenido: destino.bytes(),
            elementos: Vec::new(),
        };
    };
    let elementos = match Documento::leer(&contenido) {
        Some(despues) => claves_cambiadas(destino, &despues, &regla.variable),
        None => return Aplicacion::NoAplicable(MotivoNoAplicable::DestinoIlegible),
    };
    Aplicacion::Hecha {
        contenido,
        elementos,
    }
}

/// Las claves de primer nivel de `variable` que cambian, o la variable entera si no existía o no es una tabla.
fn claves_cambiadas(antes: &Documento, despues: &Documento, variable: &str) -> Vec<Elemento> {
    let valor_despues = despues.variable(variable).map(|v| v.valor);
    let valor_antes = antes.variable(variable).map(|v| v.valor);
    let (Some(tabla_antes), Some(tabla_despues)) =
        (valor_antes.and_then(tabla), valor_despues.and_then(tabla))
    else {
        let bytes = |documento: &Documento, valor: Option<&Expression>| {
            valor.map(|v| documento.bytes_de(v))
        };
        let cambio = match (bytes(antes, valor_antes), bytes(despues, valor_despues)) {
            (None, Some(_)) => Cambio::Anadido,
            (a, d) if a != d => Cambio::Cambiado,
            _ => return Vec::new(),
        };
        return vec![elemento(variable, cambio, Clase::Variable)];
    };
    campos(tabla_despues)
        .filter_map(|(clave, nuevo)| {
            let cambio = match campo(tabla_antes, &clave) {
                None => Cambio::Anadido,
                Some(viejo) if antes.bytes_de(viejo) != despues.bytes_de(nuevo) => Cambio::Cambiado,
                Some(_) => return None,
            };
            let nombre = format!("{variable}.{}", a_utf8(&clave).unwrap_or(clave));
            Some(elemento(&nombre, cambio, Clase::Clave))
        })
        .collect()
}

#[cfg(test)]
mod tests;
