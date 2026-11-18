//! Qué trae cada entrada del paquete, en palabras del jugador (spec 026, decisión 5). Solo sirve para enseñar: se
//! calcula desde la pieza tal como viene en el paquete y no decide nada de lo que se escribe.

use super::elementos::{bind, etiquetas_macros, lineas};
use crate::paquetes::Categoria;
use crate::piezas::{clave_de_opcion, lineas_addons, motivo_de_opcion};
use serde::Serialize;

/// Una ventana de chat con nombre: `NAME <nombre>` [V: solo aparece en los bloques `WINDOW`; 10 ventanas, 4 con
/// nombre]. Las que no lo tienen no se usan.
const NOMBRE_VENTANA: &str = "NAME ";
const ACTIVADO: &str = "enabled";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "tipo")]
pub enum Contenido {
    /// Opciones, macros, teclas (`tecla → acción`) o pestañas de chat, en el orden del fichero.
    Lista { elementos: Vec<String> },
    /// Addons que el fichero deja activados y desactivados.
    Addons {
        activados: Vec<String>,
        desactivados: Vec<String>,
    },
}

/// Lo que trae una pieza, o nada si su categoría no tiene lista o la lista sale vacía.
pub fn contenido(categoria: Categoria, pieza: &[u8]) -> Option<Contenido> {
    let elementos = match categoria {
        Categoria::Opciones => opciones(pieza),
        // Una macro sin nombre y con la primera línea en blanco no tiene nada que enseñar.
        Categoria::Macros => etiquetas_macros(pieza)
            .into_iter()
            .filter(|etiqueta| !etiqueta.is_empty())
            .collect(),
        Categoria::Teclas => teclas(pieza),
        Categoria::Chat => ventanas_de_chat(pieza),
        Categoria::AddonsActivos => return addons(pieza),
        Categoria::TeclasClic
        | Categoria::Ventanas
        | Categoria::MarcosBanda
        | Categoria::ConfigAddon => return None,
    };
    (!elementos.is_empty()).then_some(Contenido::Lista { elementos })
}

/// Las claves que se importan: las de la lista negra se apartan y se enseñan aparte.
fn opciones(pieza: &[u8]) -> Vec<String> {
    lineas(pieza)
        .iter()
        .filter_map(|l| clave_de_opcion(l))
        .filter(|clave| motivo_de_opcion(clave).is_none())
        .collect()
}

fn teclas(pieza: &[u8]) -> Vec<String> {
    lineas(pieza)
        .iter()
        .filter_map(|l| bind(l))
        .map(|(tecla, accion)| format!("{tecla} → {accion}"))
        .collect()
}

fn ventanas_de_chat(pieza: &[u8]) -> Vec<String> {
    lineas(pieza)
        .iter()
        .filter_map(|l| l.strip_prefix(NOMBRE_VENTANA))
        .map(str::trim)
        .filter(|nombre| !nombre.is_empty())
        .map(str::to_owned)
        .collect()
}

/// `AddOns.txt` solo trae `enabled` o `disabled` [V]: lo que no es `enabled` no se activa.
fn addons(pieza: &[u8]) -> Option<Contenido> {
    let mut activados = Vec::new();
    let mut desactivados = Vec::new();
    for (carpeta, valor) in lineas_addons(pieza) {
        if valor.eq_ignore_ascii_case(ACTIVADO) {
            activados.push(carpeta);
        } else {
            desactivados.push(carpeta);
        }
    }
    (!activados.is_empty() || !desactivados.is_empty()).then_some(Contenido::Addons {
        activados,
        desactivados,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lista(categoria: Categoria, pieza: &[u8]) -> Vec<String> {
        match contenido(categoria, pieza) {
            Some(Contenido::Lista { elementos }) => elementos,
            otro => panic!("se esperaba una lista: {otro:?}"),
        }
    }

    #[test]
    fn las_opciones_son_sus_claves_sin_las_que_se_apartan() {
        let pieza =
            b"SET cameraDistanceMaxZoomFactor \"2.6\"\r\nSET AllowDangerousScripts \"1\"\r\n\
SET trackedQuests \"1\"\r\nSET autoLootDefault \"1\"\r\n";
        assert_eq!(
            lista(Categoria::Opciones, pieza),
            ["cameraDistanceMaxZoomFactor", "autoLootDefault"]
        );
    }

    #[test]
    fn las_macros_sin_nombre_se_ven_por_su_primera_linea() {
        let pieza = b"VER 3 0100000000000001 \" \" \"134400\"\r\n#showtooltip\r\n/cast Corte de viento\r\nEND\r\n\
VER 3 0100000000000002 \"Monta\" \"132261\"\r\n/run Mount()\r\nEND\r\n";
        assert_eq!(lista(Categoria::Macros, pieza), ["#showtooltip", "Monta"]);
        let sin_nada = b"VER 3 01 \" \" \"1\"

END
";
        assert_eq!(contenido(Categoria::Macros, sin_nada), None);
    }

    #[test]
    fn las_teclas_son_tecla_y_accion_sin_las_lineas_que_no_son_bind() {
        let pieza = b"BINDINGMODE 0\r\nbind Q ACTIONBUTTON1\r\nbind F2 CLICK DominosActionButton109:HOTKEY\r\n";
        assert_eq!(
            lista(Categoria::Teclas, pieza),
            [
                "Q → ACTIONBUTTON1",
                "F2 → CLICK DominosActionButton109:HOTKEY"
            ]
        );
    }

    #[test]
    fn el_chat_son_las_ventanas_con_nombre() {
        let pieza = b"VERSION 9\n\nWINDOW 1\nNAME General\nSIZE 0\nEND\n\nWINDOW 2\nNAME Voz\nEND\n\nWINDOW 5\nSIZE 0\nEND\n";
        assert_eq!(lista(Categoria::Chat, pieza), ["General", "Voz"]);
    }

    #[test]
    fn los_addons_se_separan_en_activados_y_desactivados() {
        let pieza = b"Dominos: enabled\r\nDetails: disabled\r\nBlizzard_Console: enabled\r\n";
        assert_eq!(
            contenido(Categoria::AddonsActivos, pieza),
            Some(Contenido::Addons {
                activados: vec!["Dominos".into(), "Blizzard_Console".into()],
                desactivados: vec!["Details".into()],
            })
        );
    }

    #[test]
    fn las_categorias_sin_lista_y_las_listas_vacias_no_traen_contenido() {
        assert_eq!(contenido(Categoria::Ventanas, b"SET x \"1\"\r\n"), None);
        assert_eq!(contenido(Categoria::TeclasClic, b"bind Q X\r\n"), None);
        assert_eq!(contenido(Categoria::MarcosBanda, b"algo\r\n"), None);
        assert_eq!(contenido(Categoria::Chat, b"WINDOW 5\nEND\n"), None);
        assert_eq!(contenido(Categoria::AddonsActivos, b""), None);
    }
}
