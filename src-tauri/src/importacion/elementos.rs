//! Qué cambia dentro de un fichero, elemento a elemento, para la vista previa (`docs/flujos.md`, «Vista previa»). Solo
//! sirve para enseñar: los ficheros se escriben con los bytes del paquete o de la fusión, nunca reconstruidos desde aquí.

use serde::Serialize;
use std::collections::HashMap;

/// Largo máximo de una etiqueta: la primera línea de una macro sin nombre puede ser muy larga.
const MAX_ETIQUETA: usize = 60;
const CABECERA_MACRO: &str = "VER ";
const FIN_MACRO: &str = "END";
const BIND: &str = "bind ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Cambio {
    Anadido,
    Cambiado,
    Quitado,
}

/// Qué es un elemento de la configuración de un addon (spec 028). Sin clase, es una macro, una tecla, una opción o un
/// addon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Clase {
    /// Un perfil AceDB (`nombre` = el perfil).
    Perfil,
    /// El perfil que usa un personaje (`nombre` = su clave en `profileKeys`).
    PerfilDe,
    /// Una variable del fichero.
    Variable,
    /// Una clave de primer nivel de una variable (`nombre` = `variable.clave`).
    Clave,
}

/// Un elemento que cambia: una macro, una tecla, una opción o un addon. `antes` y `despues` son lo que se enseña de
/// cada lado (sin valor en el lado que no existe).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Elemento {
    pub nombre: String,
    pub cambio: Cambio,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub antes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub despues: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clase: Option<Clase>,
}

impl Elemento {
    /// El cambio entre dos valores de una misma clave, o nada si son iguales.
    pub fn entre(nombre: &str, antes: Option<&str>, despues: Option<&str>) -> Option<Self> {
        let cambio = match (antes, despues) {
            (None, Some(_)) => Cambio::Anadido,
            (Some(_), None) => Cambio::Quitado,
            (Some(a), Some(d)) if a != d => Cambio::Cambiado,
            _ => return None,
        };
        Some(Self {
            nombre: nombre.to_owned(),
            cambio,
            antes: antes.map(str::to_owned),
            despues: despues.map(str::to_owned),
            clase: None,
        })
    }

    /// El mismo elemento, de una clase de la configuración de addons.
    pub fn de_clase(self, clase: Clase) -> Self {
        Self {
            clase: Some(clase),
            ..self
        }
    }
}

/// Las líneas de un fichero de texto, sin el salto de línea.
pub(super) fn lineas(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Una macro de `macros-cache.txt`: `VER 3 <id> "<nombre>" "<icono>"`, el cuerpo y `END` [V].
#[derive(Debug, Clone, PartialEq, Eq)]
struct Macro {
    id: String,
    etiqueta: String,
    /// El bloque entero, para saber si cambió.
    bloque: String,
}

/// Las macros por id. El nombre no sirve para identificarlas: puede estar en blanco y repetido [V: 14 macros de un
/// personaje se llaman `" "`]. Lo que no está dentro de un bloque se ignora.
fn macros(bytes: &[u8]) -> Vec<Macro> {
    let mut resultado = Vec::new();
    let mut actual: Option<(String, String, Vec<String>)> = None;
    for linea in lineas(bytes) {
        if let Some(resto) = linea.strip_prefix(CABECERA_MACRO) {
            let id = resto
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_owned();
            let nombre = linea
                .split('"')
                .nth(1)
                .unwrap_or_default()
                .trim()
                .to_owned();
            actual = Some((id, nombre, vec![linea]));
        } else if let Some((id, nombre, mut bloque)) = actual.take() {
            if linea.trim() == FIN_MACRO {
                resultado.push(cerrar_macro(id, nombre, &bloque));
            } else {
                bloque.push(linea);
                actual = Some((id, nombre, bloque));
            }
        }
    }
    resultado
}

fn cerrar_macro(id: String, nombre: String, bloque: &[String]) -> Macro {
    let etiqueta = if nombre.is_empty() {
        bloque
            .get(1)
            .map(|l| l.trim().to_owned())
            .unwrap_or_default()
    } else {
        nombre
    };
    Macro {
        id,
        etiqueta: etiqueta.chars().take(MAX_ETIQUETA).collect(),
        bloque: bloque.join("\n"),
    }
}

/// Lo que se enseña de cada macro, en el orden del fichero: su nombre, o su primera línea si está en blanco.
pub(super) fn etiquetas_macros(bytes: &[u8]) -> Vec<String> {
    macros(bytes).into_iter().map(|m| m.etiqueta).collect()
}

/// Macros añadidas, cambiadas y quitadas al pasar de `antes` a `despues`, por id.
pub fn diferencias_macros(antes: &[u8], despues: &[u8]) -> Vec<Elemento> {
    let viejas: HashMap<String, Macro> = macros(antes)
        .into_iter()
        .map(|m| (m.id.clone(), m))
        .collect();
    let nuevas = macros(despues);
    let mut cambios: Vec<Elemento> = nuevas
        .iter()
        .filter_map(|nueva| match viejas.get(&nueva.id) {
            None => Elemento::entre(&nueva.etiqueta, None, Some(&nueva.etiqueta)),
            Some(vieja) if vieja.bloque != nueva.bloque => Some(Elemento {
                nombre: nueva.etiqueta.clone(),
                cambio: Cambio::Cambiado,
                antes: Some(vieja.etiqueta.clone()),
                despues: Some(nueva.etiqueta.clone()),
                clase: None,
            }),
            Some(_) => None,
        })
        .collect();
    let ids_nuevos: Vec<&str> = nuevas.iter().map(|m| m.id.as_str()).collect();
    let mut quitadas: Vec<&Macro> = viejas
        .values()
        .filter(|m| !ids_nuevos.contains(&m.id.as_str()))
        .collect();
    quitadas.sort_by(|a, b| a.id.cmp(&b.id));
    cambios.extend(
        quitadas
            .into_iter()
            .filter_map(|m| Elemento::entre(&m.etiqueta, Some(&m.etiqueta), None)),
    );
    cambios
}

/// `bind <tecla> <acción>` [V]: la tecla y la acción, o nada si la línea no es `bind`.
pub(super) fn bind(linea: &str) -> Option<(String, String)> {
    let (tecla, accion) = linea.strip_prefix(BIND)?.split_once(' ')?;
    Some((tecla.to_owned(), accion.trim().to_owned()))
}

/// Las teclas de un fichero. Una línea que no es `bind` se compara entera.
fn teclas(bytes: &[u8]) -> Vec<(String, String)> {
    lineas(bytes)
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .map(|linea| bind(&linea).unwrap_or_else(|| (linea.clone(), linea)))
        .collect()
}

/// Teclas añadidas, cambiadas y quitadas al pasar de `antes` a `despues`, en el orden del fichero nuevo.
pub fn diferencias_teclas(antes: &[u8], despues: &[u8]) -> Vec<Elemento> {
    diferencias_por_clave(&teclas(antes), &teclas(despues))
}

/// Diferencias entre dos listas `clave → valor`: primero las de `despues` en su orden, después las quitadas.
pub fn diferencias_por_clave(
    antes: &[(String, String)],
    despues: &[(String, String)],
) -> Vec<Elemento> {
    let viejas: HashMap<&str, &str> = antes
        .iter()
        .map(|(c, v)| (c.as_str(), v.as_str()))
        .collect();
    let nuevas: HashMap<&str, &str> = despues
        .iter()
        .map(|(c, v)| (c.as_str(), v.as_str()))
        .collect();
    let cambiadas = despues.iter().filter_map(|(clave, valor)| {
        Elemento::entre(clave, viejas.get(clave.as_str()).copied(), Some(valor))
    });
    let quitadas = antes
        .iter()
        .filter(|(clave, _)| !nuevas.contains_key(clave.as_str()))
        .filter_map(|(clave, valor)| Elemento::entre(clave, Some(valor), None));
    cambiadas.chain(quitadas).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Como los medidos: CRLF, nombres en blanco y repetidos.
    const ANTES: &[u8] = b"VER 3 0100000000000001 \" \" \"134400\"\r\n#showtooltip\r\n/cast Corte de viento\r\nEND\r\n\
VER 3 0100000000000002 \" \" \"134400\"\r\n/cast [target=focus] Maleficio\r\nEND\r\n\
VER 3 0100000000000003 \"Monta\" \"132261\"\r\n/run Mount()\r\nEND\r\n";
    const DESPUES: &[u8] = b"VER 3 0100000000000001 \" \" \"134400\"\r\n#showtooltip\r\n/cast Corte de viento\r\nEND\r\n\
VER 3 0100000000000002 \"Foco\" \"134400\"\r\n/cast [@focus] Maleficio\r\nEND\r\n\
VER 3 0100000000000004 \" \" \"1\"\r\n/say hola\r\nEND\r\n";

    #[test]
    fn las_macros_se_comparan_por_id_y_sin_nombre_se_ven_por_su_primera_linea() {
        let cambios = diferencias_macros(ANTES, DESPUES);
        let resumen: Vec<_> = cambios
            .iter()
            .map(|c| (c.cambio, c.nombre.as_str(), c.antes.as_deref()))
            .collect();
        assert_eq!(
            resumen,
            [
                (
                    Cambio::Cambiado,
                    "Foco",
                    Some("/cast [target=focus] Maleficio")
                ),
                (Cambio::Anadido, "/say hola", None),
                (Cambio::Quitado, "Monta", Some("Monta")),
            ]
        );
    }

    #[test]
    fn el_mismo_fichero_no_tiene_cambios() {
        assert!(diferencias_macros(ANTES, ANTES).is_empty());
        assert!(diferencias_teclas(b"bind Q X\r\n", b"bind Q X\n").is_empty());
    }

    #[test]
    fn una_etiqueta_larga_se_recorta() {
        let larga = format!(
            "VER 3 01 \" \" \"1\"\r\n/say {}\r\nEND\r\n",
            "a".repeat(100)
        );
        let cambios = diferencias_macros(b"", larga.as_bytes());
        assert_eq!(cambios[0].nombre.chars().count(), MAX_ETIQUETA);
    }

    #[test]
    fn las_teclas_se_comparan_tecla_a_tecla() {
        let antes = b"bind Q MULTIACTIONBAR3BUTTON6\r\nbind E JUMP\r\nbind F2 CLICK DominosActionButton109:HOTKEY\r\n";
        let despues = b"bind Q ACTIONBUTTON1\r\nbind F2 CLICK DominosActionButton109:HOTKEY\r\nbind R AUTORUN\r\n";
        let cambios = diferencias_teclas(antes, despues);
        let resumen: Vec<_> = cambios
            .iter()
            .map(|c| {
                (
                    c.cambio,
                    c.nombre.as_str(),
                    c.antes.as_deref(),
                    c.despues.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            resumen,
            [
                (
                    Cambio::Cambiado,
                    "Q",
                    Some("MULTIACTIONBAR3BUTTON6"),
                    Some("ACTIONBUTTON1")
                ),
                (Cambio::Anadido, "R", None, Some("AUTORUN")),
                (Cambio::Quitado, "E", Some("JUMP"), None),
            ]
        );
    }
}
