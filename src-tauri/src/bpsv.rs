//! Formato de `.build.info` y `.flavor.info` (BPSV de Blizzard), medido en una instalación real:
//!
//! ```text
//! Branch!STRING:0|Active!DEC:1|…|Version!STRING:0|KeyRing!HEX:16|Product!STRING:0
//! eu|1|…|12.1.5.70783|3ca5…|wow
//! ```
//!
//! La primera línea da las columnas (`Nombre!TIPO:tamaño`); cada línea siguiente es una fila separada por `|`.
//! Las líneas que empiezan por `##` son comentarios.

use std::collections::BTreeMap;

/// Una fila: nombre de columna → valor.
pub type Fila<'a> = BTreeMap<&'a str, &'a str>;

/// Lee todas las filas. Sin cabecera no hay filas; una fila con menos valores que columnas deja fuera las que faltan.
pub fn leer(texto: &str) -> Vec<Fila<'_>> {
    let mut lineas = texto
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty() && !l.starts_with("##"));
    let Some(cabecera) = lineas.next() else {
        return Vec::new();
    };
    let columnas: Vec<&str> = cabecera
        .split('|')
        .map(|c| c.split('!').next().unwrap_or_default())
        .collect();
    lineas
        .map(|linea| columnas.iter().copied().zip(linea.split('|')).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILD_INFO: &str = "Branch!STRING:0|Active!DEC:1|Version!STRING:0|Product!STRING:0\n\
                              eu|1|12.1.5.70783|wow\n\
                              eu|1|1.60.1.70938|wowf\n";

    #[test]
    fn lee_columnas_y_filas() {
        let filas = leer(BUILD_INFO);
        assert_eq!(filas.len(), 2);
        assert_eq!(filas[1]["Product"], "wowf");
        assert_eq!(filas[1]["Version"], "1.60.1.70938");
    }

    #[test]
    fn lee_el_flavor_info_con_retornos_de_windows() {
        let filas = leer("Product Flavor!STRING:0\r\nwowf\r\n");
        assert_eq!(filas[0]["Product Flavor"], "wowf");
    }

    #[test]
    fn salta_comentarios_y_lineas_vacias() {
        let filas = leer("A!STRING:0\n## seqn = 1\n\nx\n");
        assert_eq!(filas.len(), 1);
        assert_eq!(filas[0]["A"], "x");
    }

    #[test]
    fn sin_cabecera_no_hay_filas() {
        assert!(leer("").is_empty());
    }

    #[test]
    fn una_fila_corta_no_inventa_valores() {
        let filas = leer("A!STRING:0|B!STRING:0\nsolo-a\n");
        assert_eq!(filas[0].get("B"), None);
    }
}
