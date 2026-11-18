//! Agente de Battle.net (spec 020): las raíces de WoW que el agente apunta en `product.db`. Es una pista para
//! Detección, que no da ninguna por buena sin `.build.info` en disco. Solo lee.
//!
//! El fichero es protobuf sin esquema publicado. Medido en un equipo real: el campo `1`, repetido, es un producto
//! instalado; dentro, `2` es el código de producto (`wow`, `wowf`…) y `3` sus ajustes de instalación, cuyo campo `1` es
//! la ruta raíz con `/`. Basta un lector mínimo del formato de cable: varints y campos con longitud.

use std::path::{Path, PathBuf};

const CAMPO_PRODUCTO: u64 = 1;
const CAMPO_CODIGO: u64 = 2;
const CAMPO_INSTALACION: u64 = 3;
const CAMPO_RUTA: u64 = 1;
/// Los clientes de WoW: `wow` y `wowf` medidos; los demás `wow…`, por inferencia.
const PREFIJO_WOW: &str = "wow";

/// Raíces de los productos de WoW de `product.db`, en el orden del fichero y con `\`. Si el fichero no existe, no se
/// puede leer o no se entiende, ninguna: es una pista y la detección sigue con las demás fuentes.
pub fn raices(fichero: &Path) -> Vec<PathBuf> {
    std::fs::read(fichero)
        .ok()
        .and_then(|bytes| raices_de(&bytes))
        .unwrap_or_default()
}

fn raices_de(bytes: &[u8]) -> Option<Vec<PathBuf>> {
    let mut raices = Vec::new();
    for (campo, valor) in campos(bytes)? {
        if campo != CAMPO_PRODUCTO {
            continue;
        }
        if let Some(producto) = valor
            && let Some(raiz) = raiz_de_producto(producto)
        {
            raices.push(raiz);
        }
    }
    Some(raices)
}

/// La raíz de un producto si es de WoW. Un producto que no se entiende se salta.
fn raiz_de_producto(producto: &[u8]) -> Option<PathBuf> {
    let del_producto = campos(producto)?;
    let codigo = texto(&del_producto, CAMPO_CODIGO)?;
    if !codigo.starts_with(PREFIJO_WOW) {
        return None;
    }
    let instalacion = bytes_de(&del_producto, CAMPO_INSTALACION)?;
    let ruta = texto(&campos(instalacion)?, CAMPO_RUTA)?;
    (!ruta.is_empty()).then(|| PathBuf::from(ruta.replace('/', "\\")))
}

/// Los campos de un mensaje con su número. Solo se guarda el contenido de los que llevan longitud (textos y mensajes):
/// los varints y los de tamaño fijo no hacen falta y van como `None`.
type Campos<'a> = Vec<(u64, Option<&'a [u8]>)>;

fn bytes_de<'a>(campos: &[(u64, Option<&'a [u8]>)], buscado: u64) -> Option<&'a [u8]> {
    campos
        .iter()
        .find_map(|(campo, valor)| valor.filter(|_| *campo == buscado))
}

fn texto<'a>(campos: &[(u64, Option<&'a [u8]>)], buscado: u64) -> Option<&'a str> {
    std::str::from_utf8(bytes_de(campos, buscado)?).ok()
}

/// Todos los campos de un mensaje. `None` si los bytes no son un mensaje bien formado.
fn campos(bytes: &[u8]) -> Option<Campos<'_>> {
    let mut resultado = Vec::new();
    let mut resto = bytes;
    while !resto.is_empty() {
        let (clave, tras_clave) = varint(resto)?;
        let campo = clave >> 3;
        if campo == 0 {
            return None;
        }
        let (valor, tras_valor) = valor(clave & 0b111, tras_clave)?;
        resultado.push((campo, valor));
        resto = tras_valor;
    }
    Some(resultado)
}

/// Tipos de cable de protobuf: 0 varint, 1 de 64 bits, 2 con longitud, 5 de 32 bits. Los grupos (3 y 4) están
/// obsoletos y no aparecen: si salen, el fichero no es lo esperado.
fn valor(tipo: u64, bytes: &[u8]) -> Option<(Option<&[u8]>, &[u8])> {
    match tipo {
        0 => varint(bytes).map(|(_, resto)| (None, resto)),
        1 => bytes.get(8..).map(|resto| (None, resto)),
        2 => {
            let (largo, resto) = varint(bytes)?;
            let largo = usize::try_from(largo).ok()?;
            let contenido = resto.get(..largo)?;
            Some((Some(contenido), &resto[largo..]))
        }
        5 => bytes.get(4..).map(|resto| (None, resto)),
        _ => None,
    }
}

/// Un varint: 7 bits por byte, el bit alto dice si sigue. Como mucho 10 bytes para 64 bits.
fn varint(bytes: &[u8]) -> Option<(u64, &[u8])> {
    let mut valor = 0u64;
    for (i, byte) in bytes.iter().take(10).enumerate() {
        valor |= u64::from(byte & 0x7F) << (7 * i);
        if byte & 0x80 == 0 {
            return Some((valor, &bytes[i + 1..]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Codificador mínimo para fabricar ficheros de prueba con la forma medida.
    fn con_longitud(campo: u64, contenido: &[u8]) -> Vec<u8> {
        let mut salida = vec![u8::try_from((campo << 3) | 2).unwrap()];
        let mut largo = contenido.len();
        loop {
            let byte = u8::try_from(largo & 0x7F).unwrap();
            largo >>= 7;
            if largo == 0 {
                salida.push(byte);
                break;
            }
            salida.push(byte | 0x80);
        }
        salida.extend_from_slice(contenido);
        salida
    }

    fn producto(codigo: &str, ruta: &str) -> Vec<u8> {
        let mut instalacion = con_longitud(CAMPO_RUTA, ruta.as_bytes());
        instalacion.extend(con_longitud(2, b"eu"));
        instalacion.extend([0x18, 0x00]); // campo 3, varint 0: como en el fichero medido.
        instalacion.extend(con_longitud(13, b"_retail_"));
        let mut cuerpo = con_longitud(1, codigo.as_bytes());
        cuerpo.extend(con_longitud(CAMPO_CODIGO, codigo.as_bytes()));
        cuerpo.extend(con_longitud(CAMPO_INSTALACION, &instalacion));
        con_longitud(CAMPO_PRODUCTO, &cuerpo)
    }

    fn fichero_medido() -> Vec<u8> {
        let mut bytes = producto("agent", "C:/ProgramData/Battle.net/Agent");
        bytes.extend(producto("bna", "C:/Juegos/Battle.net"));
        bytes.extend(producto("wow", "C:/Juegos/World of Warcraft"));
        bytes.extend(producto("wowf", "C:/Juegos/World of Warcraft"));
        // Campo 3 de primer nivel, con texto: aparece en el fichero real y no es un producto.
        bytes.extend(con_longitud(3, b"C:/Juegos/Battle.net/Battle.net.exe"));
        bytes
    }

    #[test]
    fn da_las_raices_de_wow_con_barra_invertida_y_en_orden() {
        assert_eq!(
            raices_de(&fichero_medido()).unwrap(),
            [
                PathBuf::from(r"C:\Juegos\World of Warcraft"),
                PathBuf::from(r"C:\Juegos\World of Warcraft"),
            ]
        );
    }

    #[test]
    fn sin_fichero_o_con_basura_no_da_ninguna() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(raices(&tmp.path().join("product.db")).is_empty());
        let basura = tmp.path().join("basura.db");
        std::fs::write(&basura, [0xFF; 64]).unwrap();
        assert!(raices(&basura).is_empty());
    }

    #[test]
    fn se_lee_del_fichero() {
        let tmp = tempfile::tempdir().unwrap();
        let ruta = tmp.path().join("product.db");
        std::fs::write(&ruta, fichero_medido()).unwrap();
        assert_eq!(raices(&ruta).len(), 2);
    }

    #[test]
    fn un_fichero_truncado_en_cualquier_punto_no_entra_en_panico() {
        let bytes = fichero_medido();
        for corte in 0..bytes.len() {
            let _ = raices_de(&bytes[..corte]);
        }
    }

    #[test]
    fn un_producto_sin_ruta_o_que_no_se_entiende_se_salta() {
        let mut bytes = con_longitud(CAMPO_PRODUCTO, &con_longitud(CAMPO_CODIGO, b"wow"));
        bytes.extend(con_longitud(CAMPO_PRODUCTO, &[0xFF, 0xFF]));
        bytes.extend(producto("wowf", "D:/WoW"));
        assert_eq!(raices_de(&bytes).unwrap(), [PathBuf::from(r"D:\WoW")]);
    }

    #[test]
    fn varint_de_varios_bytes_y_demasiado_largo() {
        assert_eq!(varint(&[0xAC, 0x02]).map(|(v, _)| v), Some(300));
        assert_eq!(varint(&[0x80; 11]), None);
    }
}
