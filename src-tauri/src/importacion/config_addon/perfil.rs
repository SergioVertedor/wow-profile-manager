//! El perfil AceDB al importar (spec 028, decisiones 7 y 11): el perfil y el de cada módulo, a qué perfil apunta el
//! personaje destino y qué pasa si ya hay uno con ese nombre.

use super::{AccionPerfil, Aplicacion, ConflictoPerfil, MotivoNoAplicable, elemento};
use crate::importacion::elementos::{Cambio, Clase, Elemento};
use crate::lua::{
    self, CLAVE_NAMESPACES, CLAVE_PERFIL, CLAVE_PERFILES, CLAVE_PERSONAJES, Documento,
    MARCADOR_PERSONAJE, PerfilAcedb, a_utf8, cadena, campo, campos, literal_latin1, tabla,
};
use full_moon::ast::TableConstructor;

/// Hasta dónde se busca un nombre libre al renombrar un perfil.
const MAX_RENOMBRADO: u32 = 999;

/// Lo que dice el manifiesto de un perfil AceDB, y para quién.
pub(super) struct DatosPerfil<'a> {
    pub entrada: &'a str,
    pub variable: &'a str,
    /// El nombre del perfil en el manifiesto (UTF-8, o el marcador).
    pub perfil: &'a str,
    /// La clave del personaje destino (UTF-8).
    pub clave: &'a str,
}

/// Cómo está un perfil en el destino frente al de la pieza.
#[derive(Debug, PartialEq, Eq)]
enum Estado {
    /// No está ni en `profiles` ni en ningún módulo.
    Libre,
    /// Está, y todo lo que hay es igual al de la pieza.
    Igual,
    Distinto,
}

/// Perfil AceDB (decisiones 7 y 11): el perfil, el de cada módulo y `profileKeys` del personaje.
pub(super) fn perfil_acedb(
    destino: &Documento,
    pieza: &[u8],
    datos: &DatosPerfil<'_>,
    decision: Option<AccionPerfil>,
) -> Aplicacion {
    let Some(pieza) = Documento::leer_valor(pieza) else {
        return Aplicacion::NoAplicable(MotivoNoAplicable::DestinoIlegible);
    };
    let raiz = destino
        .variable(datos.variable)
        .and_then(|v| tabla(v.valor));
    let pedido = if datos.perfil == MARCADOR_PERSONAJE {
        datos.clave
    } else {
        datos.perfil
    };
    let estado = |nombre: &str| raiz.map_or(Estado::Libre, |r| estado(destino, r, &pieza, nombre));
    let actual = estado(pedido);
    let nombre = match (&actual, decision) {
        (Estado::Distinto, Some(AccionPerfil::Renombrar)) => libre(pedido, estado),
        (Estado::Distinto, None) => {
            return Aplicacion::Conflicto(ConflictoPerfil {
                entrada: datos.entrada.to_owned(),
                perfil: pedido.to_owned(),
                propuesto: libre(pedido, estado),
                usado_por: raiz.map_or_else(Vec::new, |r| usado_por(r, pedido, datos.clave)),
            });
        }
        _ => pedido.to_owned(),
    };
    let clave_latin1 = literal_latin1(datos.clave);
    let nombre_latin1 = literal_latin1(&nombre);
    let perfil = PerfilAcedb {
        variable: datos.variable,
        pieza: &pieza,
        nombre: &nombre_latin1,
        personaje: &clave_latin1,
    };
    let Some(contenido) = lua::fusionar_perfil(destino, &perfil) else {
        return Aplicacion::NoAplicable(MotivoNoAplicable::DestinoIlegible);
    };
    let mut elementos = Vec::new();
    let cambio_perfil = match estado(&nombre) {
        Estado::Libre => Some(Cambio::Anadido),
        Estado::Distinto => Some(Cambio::Cambiado),
        Estado::Igual => None,
    };
    elementos.extend(cambio_perfil.map(|c| elemento(&nombre, c, Clase::Perfil)));
    let antes = raiz.and_then(|r| perfil_de(r, &clave_latin1));
    elementos.extend(
        Elemento::entre(datos.clave, antes.as_deref(), Some(&nombre))
            .map(|e| e.de_clase(Clase::PerfilDe)),
    );
    Aplicacion::Hecha {
        contenido,
        elementos,
    }
}

/// Cómo está el perfil `nombre` (UTF-8) en la variable del destino frente al de la pieza y sus módulos.
fn estado(destino: &Documento, raiz: &TableConstructor, pieza: &Documento, nombre: &str) -> Estado {
    let nombre = literal_latin1(nombre);
    let de_la_pieza = pieza.valor_suelto().and_then(|v| tabla(v.valor));
    let mut pares = Vec::new();
    pares.push((
        campo(raiz, CLAVE_PERFILES).and_then(tabla),
        de_la_pieza.and_then(|t| campo(t, CLAVE_PERFIL)),
    ));
    let modulos_destino = campo(raiz, CLAVE_NAMESPACES).and_then(tabla);
    for (modulo, valor) in de_la_pieza
        .and_then(|t| campo(t, CLAVE_NAMESPACES))
        .and_then(tabla)
        .into_iter()
        .flat_map(campos)
    {
        let perfiles = modulos_destino
            .and_then(|m| campo(m, &modulo))
            .and_then(tabla)
            .and_then(|m| campo(m, CLAVE_PERFILES))
            .and_then(tabla);
        pares.push((perfiles, Some(valor)));
    }
    let mut existe = false;
    for (perfiles, de_pieza) in pares {
        let Some(actual) = perfiles.and_then(|p| campo(p, &nombre)) else {
            continue;
        };
        existe = true;
        if de_pieza.is_none_or(|p| pieza.bytes_de(p) != destino.bytes_de(actual)) {
            return Estado::Distinto;
        }
    }
    if existe { Estado::Igual } else { Estado::Libre }
}

/// El primer `<nombre> (n)` que no choca: libre, o igual al de la pieza (otro destino de la misma cuenta ya lo añadió).
fn libre(nombre: &str, estado: impl Fn(&str) -> Estado) -> String {
    (2..=MAX_RENOMBRADO)
        .map(|n| format!("{nombre} ({n})"))
        .find(|candidato| estado(candidato) != Estado::Distinto)
        .unwrap_or_else(|| format!("{nombre} ({})", MAX_RENOMBRADO + 1))
}

/// Los personajes que, además de `clave`, apuntan a `nombre` en `profileKeys`.
fn usado_por(raiz: &TableConstructor, nombre: &str, clave: &str) -> Vec<String> {
    let nombre = literal_latin1(nombre);
    let clave = literal_latin1(clave);
    campo(raiz, CLAVE_PERSONAJES)
        .and_then(tabla)
        .into_iter()
        .flat_map(campos)
        .filter(|(k, v)| *k != clave && cadena(v).as_deref() == Some(nombre.as_str()))
        .filter_map(|(k, _)| a_utf8(&k))
        .collect()
}

/// El perfil al que apunta un personaje (clave en Latin-1), en UTF-8.
fn perfil_de(raiz: &TableConstructor, clave_latin1: &str) -> Option<String> {
    campos(tabla(campo(raiz, CLAVE_PERSONAJES)?)?)
        .find(|(k, _)| k == clave_latin1)
        .and_then(|(_, v)| cadena(v))
        .and_then(|v| a_utf8(&v))
}
