//! Fusiones línea a línea (#63): para no pisar lo que el destino ya tiene, cambian solo las claves o los addons que trae el paquete y
//! el resto del fichero del destino se conserva byte a byte.

use super::elementos::{Cambio, Elemento};
use crate::piezas::{
    ACTIVADO, DESACTIVADO, Motivo, PREFIJO_BLIZZARD, Quitadas, addon_de_linea, agrupar,
    clave_de_opcion, motivo_de_opcion,
};
use std::collections::{HashMap, HashSet};

const CRLF: &[u8] = b"\r\n";
const LF: &[u8] = b"\n";
const BOM: &[u8] = b"\xef\xbb\xbf";

/// El resultado de una fusión: los bytes nuevos y qué cambia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fusion {
    pub contenido: Vec<u8>,
    pub cambios: Vec<Elemento>,
    /// Opciones: claves del paquete que no se importan (lista negra), por motivo. Addons: carpetas que se activan y no
    /// están instaladas en el destino.
    pub apartadas: Vec<Quitadas>,
}

/// Una línea del paquete que sustituye o se añade: su clave para comparar (en minúsculas), su nombre tal como lo
/// escribe el paquete, el texto que se escribe y el valor que se enseña.
struct Linea {
    clave: String,
    nombre: String,
    texto: String,
    valor: String,
}

/// `config-cache.wtf`: por clave `SET`, sin distinguir mayúsculas (los CVar no las distinguen). Las claves de la lista
/// negra se descartan también aquí: un paquete hecho a mano podría traer `AllowDangerousScripts`.
pub fn fusionar_opciones(destino: &[u8], paquete: &[u8]) -> Fusion {
    let mut descartadas = Vec::new();
    let lineas: Vec<Linea> = texto_lineas(paquete)
        .filter_map(|texto| {
            let clave = clave_de_opcion(&texto)?;
            if let Some(motivo) = motivo_de_opcion(&clave) {
                descartadas.push((motivo, clave));
                return None;
            }
            Some(Linea {
                clave: clave.to_lowercase(),
                nombre: clave,
                valor: valor_de_opcion(&texto),
                texto,
            })
        })
        .collect();
    let mut fusion = fusionar(
        destino,
        &lineas,
        |texto| clave_de_opcion(texto).map(|c| (c.to_lowercase(), valor_de_opcion(texto))),
        |_, nueva| nueva.texto.clone(),
    );
    fusion.apartadas = agrupar(descartadas);
    fusion
}

/// `AddOns.txt`: por carpeta, sin distinguir mayúsculas. Se conserva cómo escribe el destino la carpeta y solo cambia
/// el valor. Las carpetas que el destino no tiene instaladas se escriben igual (#63, decisión 9) y se dicen. Una
/// línea con un valor que no sea `enabled` o `disabled` no se escribe: el juego no escribe otros [V].
pub fn fusionar_addons(destino: &[u8], paquete: &[u8], instalados: &HashSet<String>) -> Fusion {
    let lineas: Vec<Linea> = texto_lineas(paquete)
        .filter_map(|texto| {
            let (carpeta, valor) = addon_de_linea(&texto)?;
            let conocido = [ACTIVADO, DESACTIVADO]
                .iter()
                .any(|v| valor.eq_ignore_ascii_case(v));
            conocido.then(|| Linea {
                clave: carpeta.to_lowercase(),
                nombre: carpeta.to_owned(),
                texto: format!("{carpeta}: {valor}"),
                valor: valor.to_owned(),
            })
        })
        .collect();
    let faltan = agrupar(
        lineas
            .iter()
            .filter(|l| {
                !l.clave.starts_with(PREFIJO_BLIZZARD)
                    && !instalados.contains(&l.clave)
                    && l.valor.eq_ignore_ascii_case(ACTIVADO)
            })
            .map(|l| (Motivo::AddonNoInstalado, l.nombre.clone())),
    );
    let mut fusion = fusionar(
        destino,
        &lineas,
        |texto| addon_de_linea(texto).map(|(c, v)| (c.to_lowercase(), v.to_owned())),
        |texto, nueva| {
            let carpeta = addon_de_linea(texto).map_or(nueva.clave.as_str(), |(c, _)| c);
            format!("{carpeta}: {}", nueva.valor)
        },
    );
    fusion.apartadas = faltan;
    fusion
}

/// Sustituye cada línea del destino cuya clave trae el paquete y añade al final las que el destino no tiene, en el
/// orden del paquete. Las líneas sustituidas conservan el salto de línea (y el BOM) del destino; las añadidas usan el
/// suyo. `reescribir` da el texto de una línea sustituida a partir de la del destino y la del paquete.
fn fusionar(
    destino: &[u8],
    paquete: &[Linea],
    clave_y_valor: impl Fn(&str) -> Option<(String, String)>,
    reescribir: impl Fn(&str, &Linea) -> String,
) -> Fusion {
    // Si el paquete repite una clave, gana la primera, igual que al añadir las que el destino no tiene.
    let mut nuevas: HashMap<&str, &Linea> = HashMap::new();
    for linea in paquete {
        nuevas.entry(linea.clave.as_str()).or_insert(linea);
    }
    let mut contenido = Vec::with_capacity(destino.len());
    let mut cambios = Vec::new();
    let mut vistas = HashSet::new();
    for linea in destino.split_inclusive(|b| *b == b'\n') {
        let texto = String::from_utf8_lossy(linea);
        let Some((clave, valor)) = clave_y_valor(&texto) else {
            contenido.extend_from_slice(linea);
            continue;
        };
        let Some(nueva) = nuevas.get(clave.as_str()) else {
            contenido.extend_from_slice(linea);
            continue;
        };
        if linea.starts_with(BOM) {
            contenido.extend_from_slice(BOM);
        }
        contenido.extend_from_slice(reescribir(texto.trim_end(), nueva).as_bytes());
        contenido.extend_from_slice(salto_de(linea));
        if vistas.insert(clave.clone()) {
            cambios.extend(Elemento::entre(
                &nueva.nombre,
                Some(&valor),
                Some(&nueva.valor),
            ));
        }
    }
    let salto = salto_preferido(destino);
    for nueva in paquete {
        // Ya estaba en el destino, o el paquete la repite: se escribe una sola vez.
        if !vistas.insert(nueva.clave.clone()) {
            continue;
        }
        if !contenido.is_empty() && !contenido.ends_with(LF) {
            contenido.extend_from_slice(salto);
        }
        contenido.extend_from_slice(nueva.texto.as_bytes());
        contenido.extend_from_slice(salto);
        cambios.push(Elemento {
            nombre: nueva.nombre.clone(),
            cambio: Cambio::Anadido,
            antes: None,
            despues: Some(nueva.valor.clone()),
            clase: None,
        });
    }
    Fusion {
        contenido,
        cambios,
        apartadas: Vec::new(),
    }
}

/// Las líneas de un fichero del paquete, sin el salto de línea.
fn texto_lineas(bytes: &[u8]) -> impl Iterator<Item = String> + '_ {
    bytes
        .split(|b| *b == b'\n')
        .map(|l| String::from_utf8_lossy(l.strip_suffix(b"\r").unwrap_or(l)).into_owned())
        .filter(|l| !l.trim().is_empty())
}

/// `SET <clave> "<valor>"`: el valor sin comillas.
fn valor_de_opcion(texto: &str) -> String {
    let resto = texto
        .trim()
        .splitn(3, char::is_whitespace)
        .nth(2)
        .unwrap_or_default()
        .trim();
    resto
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .unwrap_or(resto)
        .to_owned()
}

fn salto_de(linea: &[u8]) -> &'static [u8] {
    if linea.ends_with(CRLF) {
        CRLF
    } else if linea.ends_with(LF) {
        LF
    } else {
        b""
    }
}

/// CRLF salvo que el destino use solo LF: los ficheros medidos del `WTF` son CRLF [V].
fn salto_preferido(destino: &[u8]) -> &'static [u8] {
    let lf = destino.iter().filter(|b| **b == b'\n').count();
    let crlf = destino.windows(2).filter(|v| *v == CRLF).count();
    if lf > 0 && crlf == 0 { LF } else { CRLF }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piezas::todas_las_claves;

    fn resumen(fusion: &Fusion) -> Vec<(Cambio, &str, Option<&str>, Option<&str>)> {
        fusion
            .cambios
            .iter()
            .map(|c| {
                (
                    c.cambio,
                    c.nombre.as_str(),
                    c.antes.as_deref(),
                    c.despues.as_deref(),
                )
            })
            .collect()
    }

    #[test]
    fn las_opciones_del_paquete_cambian_o_se_anaden_y_el_resto_se_conserva() {
        let destino = b"SET nameplateMaxScale \"1\"\r\nSET hardTrackedQuests \"1 2\"\r\nSET AllowDangerousScripts \"0\"\r\nSET showTimestamps \"none\"\r\n";
        let paquete = b"SET NAMEPLATEMAXSCALE \"1.2\"\r\nSET autoLootDefault \"1\"\r\nSET showTimestamps \"none\"\r\n";
        let fusion = fusionar_opciones(destino, paquete);
        assert_eq!(
            fusion.contenido,
            b"SET NAMEPLATEMAXSCALE \"1.2\"\r\nSET hardTrackedQuests \"1 2\"\r\nSET AllowDangerousScripts \"0\"\r\nSET showTimestamps \"none\"\r\nSET autoLootDefault \"1\"\r\n"
        );
        assert_eq!(
            resumen(&fusion),
            [
                (
                    Cambio::Cambiado,
                    "NAMEPLATEMAXSCALE",
                    Some("1"),
                    Some("1.2")
                ),
                (Cambio::Anadido, "autoLootDefault", None, Some("1")),
            ]
        );
    }

    #[test]
    fn al_importar_se_vuelve_a_aplicar_la_lista_negra() {
        let destino = b"SET AllowDangerousScripts \"0\"\r\n";
        let paquete =
            b"SET AllowDangerousScripts \"1\"\r\nSET trackedQuests \"9\"\r\nSET a \"1\"\r\n";
        let fusion = fusionar_opciones(destino, paquete);
        assert_eq!(
            fusion.contenido,
            b"SET AllowDangerousScripts \"0\"\r\nSET a \"1\"\r\n"
        );
        assert_eq!(
            todas_las_claves(&fusion.apartadas),
            ["AllowDangerousScripts", "trackedQuests"]
        );
        assert_eq!(fusion.apartadas[1].motivo, Motivo::Estado);
    }

    #[test]
    fn con_un_paquete_vacio_el_destino_queda_byte_a_byte() {
        let destino = b"\xef\xbb\xbfSET a \"1\"\r\nlinea rara\nSET b \"2\"";
        assert_eq!(fusionar_opciones(destino, b"").contenido, destino);
        let addons = b"Dominos: enabled\r\nDetails: disabled";
        assert_eq!(
            fusionar_addons(addons, b"", &HashSet::new()).contenido,
            addons
        );
    }

    #[test]
    fn sin_salto_final_lo_anadido_va_en_su_propia_linea() {
        let fusion = fusionar_opciones(b"SET a \"1\"", b"SET b \"2\"\r\n");
        assert_eq!(fusion.contenido, b"SET a \"1\"\r\nSET b \"2\"\r\n");
        let solo_lf = fusionar_opciones(b"SET a \"1\"\n", b"SET b \"2\"\r\n");
        assert_eq!(solo_lf.contenido, b"SET a \"1\"\nSET b \"2\"\n");
        let vacio = fusionar_opciones(b"", b"SET b \"2\"\n");
        assert_eq!(vacio.contenido, b"SET b \"2\"\r\n");
    }

    #[test]
    fn una_clave_repetida_en_el_paquete_cuenta_una_vez_y_gana_la_primera() {
        let paquete = b"SET a \"1\"\r\nSET A \"2\"\r\nSET b \"3\"\r\nSET b \"4\"\r\n";
        let fusion = fusionar_opciones(b"SET a \"0\"\r\n", paquete);
        assert_eq!(fusion.contenido, b"SET a \"1\"\r\nSET b \"3\"\r\n");
        assert_eq!(fusion.cambios.len(), 2);
    }

    #[test]
    fn en_addons_solo_se_escriben_enabled_y_disabled() {
        let paquete = b"Dominos: ENABLED\r\nDetails: quizas\r\nPawn: disabled\r\n";
        let instalados = HashSet::from(["dominos".to_owned(), "pawn".to_owned()]);
        let fusion = fusionar_addons(b"", paquete, &instalados);
        assert_eq!(fusion.contenido, b"Dominos: ENABLED\r\nPawn: disabled\r\n");
    }

    #[test]
    fn los_addons_cambian_por_carpeta_sin_mayusculas_y_se_dicen_los_que_faltan() {
        let instalados = HashSet::from(["dominos".to_owned(), "details".to_owned()]);
        let destino = b"Dominos: disabled\r\nDetails: enabled\r\nPrat-3.0: enabled\r\n";
        let paquete = b"DOMINOS: enabled\r\nBlizzard_CombatText: disabled\r\nWeakAuras: enabled\r\nPawn: disabled\r\n";
        let fusion = fusionar_addons(destino, paquete, &instalados);
        assert_eq!(
            fusion.contenido,
            b"Dominos: enabled\r\nDetails: enabled\r\nPrat-3.0: enabled\r\nBlizzard_CombatText: disabled\r\nWeakAuras: enabled\r\nPawn: disabled\r\n"
                .as_slice(),
            "la carpeta se escribe como la tiene el destino"
        );
        assert_eq!(
            todas_las_claves(&fusion.apartadas),
            ["WeakAuras"],
            "solo las activadas que no están instaladas"
        );
        assert_eq!(fusion.apartadas[0].motivo, Motivo::AddonNoInstalado);
        assert_eq!(
            resumen(&fusion)[0],
            (
                Cambio::Cambiado,
                "DOMINOS",
                Some("disabled"),
                Some("enabled")
            )
        );
    }
}
