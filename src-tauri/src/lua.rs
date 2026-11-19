//! Un `.lua` de `SavedVariables` como dato (D-02, `docs/formato-wowprofile.md`: «Lua es dato»). Se parsea con
//! `full_moon` sin ejecutar nada y todo lo que sale es **byte a byte** el original: solo se escriben de nuevo las
//! llaves de las tablas que se recortan (spec 027).
//!
//! El fichero guarda bytes, no texto UTF-8: se lee como Latin-1 (1 byte = 1 `char`) para que el parser lo acepte
//! entero, y cada trozo que se copia vuelve a ser sus bytes al deshacer esa lectura.

mod fusion;

pub use fusion::{PerfilAcedb, fusionar_perfil, fusionar_variable};

use full_moon::ast::{Ast, Expression, Field, Stmt, TableConstructor, UnOp, Var};
use full_moon::node::Node;
use full_moon::tokenizer::{TokenReference, TokenType};
use full_moon::visitors::{Visit, Visitor};

/// Marcador del personaje (`docs/formato-wowprofile.md`, «Marcadores»).
pub const MARCADOR_PERSONAJE: &str = "@@WOWPROFILE:PERSONAJE@@";
/// Claves de una variable AceDB (`docs/adaptadores.md`, «Qué es AceDB»).
pub const CLAVE_PERSONAJES: &str = "profileKeys";
pub const CLAVE_PERFILES: &str = "profiles";
pub const CLAVE_NAMESPACES: &str = "namespaces";
/// La clave del perfil dentro de una pieza `perfil-acedb`.
pub const CLAVE_PERFIL: &str = "profile";
/// Cualquier clave de ese nivel.
const SEGMENTO_CUALQUIERA: &str = "*";
/// La clave del personaje.
const SEGMENTO_PERSONAJE: &str = "$PJ";
const SEPARADOR_RUTA: char = '/';
/// La variable con la que se envuelve una tabla suelta para poder parsearla.
const VARIABLE_SUELTA: &str = "_";
/// Los únicos símbolos que son datos.
const SIMBOLOS_DE_DATOS: [&str; 3] = ["true", "false", "nil"];

/// Un fichero leído. Si no parsea, no hay documento.
pub struct Documento {
    latin1: String,
    ast: Ast,
    fin_de_linea: &'static str,
}

/// `Nombre = <valor>` en el primer nivel del fichero.
pub struct Variable<'a> {
    pub nombre: String,
    pub valor: &'a Expression,
    sentencia: &'a Stmt,
}

impl Documento {
    pub fn leer(bytes: &[u8]) -> Option<Self> {
        let latin1: String = bytes.iter().map(|&b| char::from(b)).collect();
        let ast = full_moon::parse(&latin1).ok()?;
        let fin_de_linea = if bytes.windows(2).any(|v| v == b"\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        Some(Self {
            latin1,
            ast,
            fin_de_linea,
        })
    }

    /// Una tabla suelta (la pieza `perfil-acedb`) como documento de una sola variable, o `None` si no es exactamente un
    /// valor. Los bytes de sus nodos son los de `bytes`: la envoltura queda fuera de cualquier rango.
    pub fn leer_valor(bytes: &[u8]) -> Option<Self> {
        let mut envuelto = format!("{VARIABLE_SUELTA} = ").into_bytes();
        envuelto.extend(bytes);
        let documento = Self::leer(&envuelto)?;
        let bloque = documento.ast.nodes();
        let una = bloque.stmts().count() == 1 && bloque.last_stmt().is_none();
        (una && documento.variable(VARIABLE_SUELTA).is_some()).then_some(documento)
    }

    /// El valor de un documento leído con `leer_valor`.
    pub fn valor_suelto(&self) -> Option<Variable<'_>> {
        self.variable(VARIABLE_SUELTA)
    }

    /// Solo hay `Nombre = <dato>`: tablas, cadenas, números, `true`, `false` y `nil` (spec 028, decisión 1). El juego
    /// ejecuta los `SavedVariables`: una llamada, una variable o una función de un paquete ajeno sería código.
    pub fn solo_datos(&self) -> bool {
        let bloque = self.ast.nodes();
        let variables = self.variables();
        bloque.last_stmt().is_none()
            && bloque.stmts().count() == variables.len()
            && variables.iter().all(|v| es_dato(v.valor))
    }

    /// El fichero entero, tal como se leyó.
    pub fn bytes(&self) -> Vec<u8> {
        a_bytes(&self.latin1)
    }

    /// El fichero con `nombre = valor`: si la variable existe se cambia solo su valor; si no, se añade al final. Lo
    /// demás queda byte a byte.
    pub fn con_variable(&self, nombre: &str, valor: &[u8]) -> Vec<u8> {
        if let Some(variable) = self.variable(nombre)
            && let Some((inicio, fin)) = variable.valor.range()
            && let (Some(antes), Some(despues)) = (
                self.latin1.get(..inicio.bytes()),
                self.latin1.get(fin.bytes()..),
            )
        {
            let mut salida = a_bytes(antes);
            salida.extend(valor);
            salida.extend(a_bytes(despues));
            return salida;
        }
        let fin = self.fin_de_linea.as_bytes();
        let mut salida = self.bytes();
        if !salida.is_empty() && !salida.ends_with(b"\n") {
            salida.extend(fin);
        }
        salida.extend(format!("{nombre} = ").as_bytes());
        salida.extend(valor);
        salida.extend(fin);
        salida
    }

    /// Las asignaciones de una sola variable con nombre, en el orden del fichero.
    pub fn variables(&self) -> Vec<Variable<'_>> {
        self.ast
            .nodes()
            .stmts()
            .filter_map(|sentencia| {
                let Stmt::Assignment(asignacion) = sentencia else {
                    return None;
                };
                let mut variables = asignacion.variables().iter();
                let mut valores = asignacion.expressions().iter();
                let (Some(Var::Name(nombre)), None, Some(valor), None) = (
                    variables.next(),
                    variables.next(),
                    valores.next(),
                    valores.next(),
                ) else {
                    return None;
                };
                Some(Variable {
                    nombre: nombre.token().to_string(),
                    valor,
                    sentencia,
                })
            })
            .collect()
    }

    pub fn variable(&self, nombre: &str) -> Option<Variable<'_>> {
        self.variables().into_iter().find(|v| v.nombre == nombre)
    }

    /// Los bytes originales de un nodo, sin el espacio de alrededor.
    pub fn bytes_de(&self, nodo: &impl Node) -> Vec<u8> {
        let Some((inicio, fin)) = nodo.range() else {
            return Vec::new();
        };
        self.latin1
            .get(inicio.bytes()..fin.bytes())
            .map(a_bytes)
            .unwrap_or_default()
    }

    /// La sentencia entera de una variable (`Nombre = <valor>`), tal como está.
    pub fn bytes_de_variable(&self, variable: &Variable<'_>) -> Vec<u8> {
        self.bytes_de(variable.sentencia)
    }

    /// Los bytes de `nodo` con `ediciones` (posición de inicio y de fin en el documento, y lo que va en su lugar)
    /// aplicadas. Lo que no se edita queda tal cual.
    fn editado(&self, nodo: &impl Node, mut ediciones: Vec<(usize, usize, Vec<u8>)>) -> Vec<u8> {
        let Some((inicio, fin)) = nodo.range().map(|(i, f)| (i.bytes(), f.bytes())) else {
            return Vec::new();
        };
        ediciones.sort_by_key(|(i, ..)| *i);
        let mut salida = Vec::new();
        let mut cursor = inicio;
        for (desde, hasta, bytes) in ediciones {
            salida.extend(
                self.latin1
                    .get(cursor..desde)
                    .map(a_bytes)
                    .unwrap_or_default(),
            );
            salida.extend(bytes);
            cursor = hasta;
        }
        salida.extend(
            self.latin1
                .get(cursor..fin)
                .map(a_bytes)
                .unwrap_or_default(),
        );
        salida
    }

    /// Los espacios que hay al principio de la línea antes de la posición `posicion`, o nada si hay algo más.
    fn sangria(&self, posicion: usize) -> String {
        let antes = self.latin1.get(..posicion).unwrap_or_default();
        let linea = antes.rsplit('\n').next().unwrap_or_default();
        if linea.chars().all(|c| c == ' ' || c == '\t') {
            linea.to_owned()
        } else {
            String::new()
        }
    }

    pub fn fin_de_linea(&self) -> &'static str {
        self.fin_de_linea
    }

    /// `valor` recortado según `seleccion`, o `None` si no queda nada que viaje.
    pub fn recortar(&self, valor: &Expression, seleccion: &Seleccion<'_>) -> Option<Vec<u8>> {
        Recorte {
            documento: self,
            personaje: seleccion.personaje,
            salida_personaje: MARCADOR_PERSONAJE,
        }
        .valor(valor, &Rutas::de(seleccion))
    }
}

/// Qué viaja de un valor (`docs/adaptadores.md`, «Cómo se escribe un adaptador»). Las rutas van separadas por `/`.
pub struct Seleccion<'a> {
    /// `None` = todo.
    pub incluir: Option<&'a [String]>,
    pub excluir: &'a [String],
    /// Tablas indexadas por personaje: solo viaja la entrada de `personaje`.
    pub solo_personaje: &'a [String],
    /// Clave del personaje de origen tal como la escribe el addon.
    pub personaje: &'a str,
}

/// Las rutas que quedan por recorrer a partir de un nivel.
#[derive(Clone)]
struct Rutas<'a> {
    /// `None` = todo lo que hay debajo.
    incluir: Option<Vec<Vec<&'a str>>>,
    excluir: Vec<Vec<&'a str>>,
    solo: Vec<Vec<&'a str>>,
}

impl<'a> Rutas<'a> {
    fn de(seleccion: &Seleccion<'a>) -> Self {
        let partir = |rutas: &'a [String]| -> Vec<Vec<&'a str>> {
            rutas
                .iter()
                .map(|r| r.split(SEPARADOR_RUTA).collect())
                .collect()
        };
        Self {
            incluir: seleccion.incluir.map(partir),
            excluir: partir(seleccion.excluir),
            solo: partir(seleccion.solo_personaje),
        }
    }

    /// Debajo de aquí todo viaja tal cual: no hay nada que recortar.
    fn entero(&self) -> bool {
        let incluido = self
            .incluir
            .as_ref()
            .is_none_or(|rutas| rutas.iter().any(Vec::is_empty));
        incluido && self.excluir.is_empty() && self.solo.is_empty()
    }
}

struct Recorte<'d> {
    documento: &'d Documento,
    /// La clave del personaje tal como está en `documento`.
    personaje: &'d str,
    /// Cómo se escribe la clave del personaje en la salida: el marcador al exportar, la del destino al importar.
    salida_personaje: &'d str,
}

impl Recorte<'_> {
    fn valor(&self, valor: &Expression, rutas: &Rutas<'_>) -> Option<Vec<u8>> {
        if rutas.entero() {
            return Some(self.documento.bytes_de(valor));
        }
        match valor {
            Expression::TableConstructor(tabla) => self.tabla(tabla, rutas),
            // Un valor suelto solo viaja si una ruta de `incluir` termina en él.
            _ => rutas
                .incluir
                .as_ref()
                .is_none_or(|r| r.iter().any(Vec::is_empty))
                .then(|| self.documento.bytes_de(valor)),
        }
    }

    fn tabla(&self, tabla: &TableConstructor, rutas: &Rutas<'_>) -> Option<Vec<u8>> {
        let fin = self.documento.fin_de_linea;
        let todo = rutas
            .incluir
            .as_ref()
            .is_none_or(|r| r.iter().any(Vec::is_empty));
        let mut cuerpo = Vec::new();
        for campo in tabla.fields() {
            let Some((clave, valor)) = clave_y_valor(campo) else {
                // Las entradas sin clave (listas) solo viajan si viaja todo este nivel.
                if todo {
                    cuerpo.extend(self.documento.bytes_de(campo));
                    cuerpo.extend(b",");
                    cuerpo.extend(fin.as_bytes());
                }
                continue;
            };
            let Some((hijas, es_personaje)) = self.hijas(rutas, &clave) else {
                continue;
            };
            let Some(texto) = self.valor(valor, &hijas) else {
                continue;
            };
            if es_personaje {
                cuerpo.extend(clave_entre_corchetes(self.salida_personaje));
            } else {
                cuerpo.extend(self.documento.bytes_de_clave(campo));
            }
            cuerpo.extend(b" = ");
            cuerpo.extend(texto);
            cuerpo.extend(b",");
            cuerpo.extend(fin.as_bytes());
        }
        if cuerpo.is_empty() && !todo {
            return None;
        }
        let mut salida = format!("{{{fin}").into_bytes();
        salida.extend(cuerpo);
        salida.extend(b"}");
        Some(salida)
    }

    /// Las rutas para el valor de `clave`, o `None` si esa clave no viaja. El `bool` dice si la clave es la del
    /// personaje y se escribe con su marcador.
    fn hijas<'r>(&self, rutas: &Rutas<'r>, clave: &str) -> Option<(Rutas<'r>, bool)> {
        let mut es_personaje = false;
        let mut coincide = |segmento: &str| match segmento {
            SEGMENTO_CUALQUIERA => true,
            SEGMENTO_PERSONAJE => {
                let es = clave == self.personaje;
                es_personaje |= es;
                es
            }
            literal => literal == clave,
        };
        let mut siguen = |lista: &[Vec<&'r str>]| -> Vec<Vec<&'r str>> {
            lista
                .iter()
                .filter(|r| r.first().is_some_and(|s| coincide(s)))
                .map(|r| r[1..].to_vec())
                .collect()
        };
        let incluir = match &rutas.incluir {
            Some(lista) if !lista.iter().any(Vec::is_empty) => {
                let siguen = siguen(lista);
                if siguen.is_empty() {
                    return None;
                }
                Some(siguen)
            }
            _ => None,
        };
        let excluir = siguen(&rutas.excluir);
        if excluir.iter().any(Vec::is_empty) {
            return None;
        }
        // En el nivel de `$PJ` de `soloPersonaje`, las demás claves no viajan.
        let pide_personaje = rutas
            .solo
            .iter()
            .any(|r| r.first() == Some(&SEGMENTO_PERSONAJE));
        let solo = siguen(&rutas.solo);
        if pide_personaje && !es_personaje {
            return None;
        }
        let solo = solo.into_iter().filter(|r| !r.is_empty()).collect();
        Some((
            Rutas {
                incluir,
                excluir,
                solo,
            },
            es_personaje,
        ))
    }
}

impl Documento {
    /// La clave de un campo tal como está (`["k"]` o `k`).
    fn bytes_de_clave(&self, campo: &Field) -> Vec<u8> {
        match campo {
            Field::ExpressionKey { brackets, key, .. } => {
                let (abre, cierra) = brackets.tokens();
                let mut bytes = self.bytes_de(abre);
                bytes.extend(self.bytes_de(key));
                bytes.extend(self.bytes_de(cierra));
                bytes
            }
            Field::NameKey { key, .. } => self.bytes_de(key),
            _ => Vec::new(),
        }
    }
}

/// AceDB: el valor es una tabla con `profileKeys` y `profiles`.
pub fn es_acedb(valor: &Expression) -> bool {
    tabla(valor).is_some_and(|t| {
        campo(t, CLAVE_PERSONAJES).and_then(tabla).is_some()
            && campo(t, CLAVE_PERFILES).and_then(tabla).is_some()
    })
}

/// Un valor que es solo dato, a cualquier profundidad.
fn es_dato(valor: &Expression) -> bool {
    match valor {
        Expression::String(_) | Expression::Number(_) => true,
        Expression::Symbol(token) => {
            SIMBOLOS_DE_DATOS.contains(&token.token().to_string().as_str())
        }
        Expression::UnaryOperator {
            unop: UnOp::Minus(_),
            expression,
        } => matches!(**expression, Expression::Number(_)),
        Expression::TableConstructor(tabla) => tabla.fields().iter().all(|campo| match campo {
            Field::ExpressionKey { key, value, .. } => es_dato(key) && es_dato(value),
            Field::NameKey { value, .. } | Field::NoKey(value) => es_dato(value),
            _ => false,
        }),
        _ => false,
    }
}

/// La tabla de un valor, si lo es.
pub fn tabla(valor: &Expression) -> Option<&TableConstructor> {
    match valor {
        Expression::TableConstructor(tabla) => Some(tabla),
        _ => None,
    }
}

/// El valor de la clave `clave` (`["clave"]` o `clave`) de una tabla.
pub fn campo<'a>(tabla: &'a TableConstructor, clave: &str) -> Option<&'a Expression> {
    tabla
        .fields()
        .iter()
        .filter_map(clave_y_valor)
        .find(|(k, _)| k == clave)
        .map(|(_, v)| v)
}

/// Clave y valor de cada campo con clave de una tabla, con la clave todavía en Latin-1.
pub fn campos(tabla: &TableConstructor) -> impl Iterator<Item = (String, &Expression)> {
    tabla.fields().iter().filter_map(clave_y_valor)
}

/// El texto de una cadena, todavía en Latin-1 (`None` si no es una cadena).
pub fn cadena(valor: &Expression) -> Option<String> {
    match valor {
        Expression::String(token) => literal(token),
        _ => None,
    }
}

/// Clave y valor de un campo. Una clave que no es cadena ni nombre (`[1]`) se toma por su texto.
fn clave_y_valor(campo: &Field) -> Option<(String, &Expression)> {
    match campo {
        Field::ExpressionKey { key, value, .. } => {
            let clave = match key {
                Expression::String(token) => literal(token)?,
                otra => otra.to_string().trim().to_owned(),
            };
            Some((clave, value))
        }
        Field::NameKey { key, value, .. } => Some((key.token().to_string(), value)),
        _ => None,
    }
}

/// El texto de una cadena, sin comillas. Los escapes llegan sin procesar [I: ninguno medido en claves].
fn literal(token: &TokenReference) -> Option<String> {
    match token.token_type() {
        TokenType::StringLiteral { literal, .. } => Some(literal.to_string()),
        _ => None,
    }
}

/// Todas las claves de cadena de un valor, a cualquier profundidad, ya en UTF-8.
pub fn claves_de_cadena(valor: &Expression) -> Vec<String> {
    let mut buscador = ClavesDeCadena::default();
    valor.visit(&mut buscador);
    buscador.claves
}

#[derive(Default)]
struct ClavesDeCadena {
    claves: Vec<String>,
}

impl Visitor for ClavesDeCadena {
    fn visit_field(&mut self, campo: &Field) {
        if let Field::ExpressionKey {
            key: Expression::String(token),
            ..
        } = campo
        {
            self.claves.extend(literal(token).and_then(|c| a_utf8(&c)));
        }
    }
}

/// Un texto UTF-8 como lo vería el parser: cada byte, un `char` (para comparar con las claves de un documento).
pub fn a_latin1(texto: &str) -> String {
    texto.bytes().map(char::from).collect()
}

/// Un texto UTF-8 tal como va entre comillas en un fichero, en Latin-1 (spec 028, decisión 12): `\`, `"` y los
/// caracteres de control escapados (`\ddd`), para que un nombre con un salto de línea no rompa el fichero.
pub fn literal_latin1(texto: &str) -> String {
    let mut escapado = String::with_capacity(texto.len());
    for c in texto.chars() {
        match c {
            '\\' => escapado.push_str("\\\\"),
            '"' => escapado.push_str("\\\""),
            c if c.is_ascii_control() => escapado.push_str(&format!("\\{:03}", u32::from(c))),
            c => escapado.push(c),
        }
    }
    a_latin1(&escapado)
}

/// `["clave"]` con los bytes originales de una clave leída en Latin-1.
pub fn clave_entre_corchetes(latin1: &str) -> Vec<u8> {
    let mut bytes = b"[\"".to_vec();
    bytes.extend(a_bytes(latin1));
    bytes.extend(b"\"]");
    bytes
}

/// Deshace la lectura en Latin-1: cada `char` vuelve a ser su byte.
fn a_bytes(latin1: &str) -> Vec<u8> {
    latin1
        .chars()
        .filter_map(|c| u8::try_from(u32::from(c)).ok())
        .collect()
}

/// Un texto leído en Latin-1, otra vez en UTF-8 (`None` si sus bytes no lo son).
pub fn a_utf8(latin1: &str) -> Option<String> {
    String::from_utf8(a_bytes(latin1)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PJ: &str = "Goldberg Joestar";

    fn recorte(fuente: &str, variable: &str, seleccion: &Seleccion<'_>) -> Option<String> {
        let doc = Documento::leer(fuente.as_bytes()).unwrap();
        let var = doc.variable(variable).unwrap();
        doc.recortar(var.valor, seleccion)
            .map(|b| String::from_utf8(b).unwrap())
    }

    fn seleccion<'a>(
        incluir: Option<&'a [String]>,
        excluir: &'a [String],
        solo: &'a [String],
    ) -> Seleccion<'a> {
        Seleccion {
            incluir,
            excluir,
            solo_personaje: solo,
            personaje: PJ,
        }
    }

    fn rutas(lista: &[&str]) -> Vec<String> {
        lista.iter().map(|r| (*r).to_owned()).collect()
    }

    #[test]
    fn copia_byte_a_byte_tambien_latin1() {
        // `ø` en UTF-8 son dos bytes; un `ÿ` suelto (0xFF) no es UTF-8 y tiene que volver igual.
        let mut bytes = "Var = {\r\n[\"Gøldberg\"] = \"x\",\r\n[\"k\"] = \""
            .as_bytes()
            .to_vec();
        bytes.push(0xFF);
        bytes.extend(b"\",\r\n}\r\n");
        let doc = Documento::leer(&bytes).unwrap();
        let var = doc.variable("Var").unwrap();
        let esperado = &bytes[6..bytes.len() - 2];
        assert_eq!(doc.bytes_de(var.valor), esperado);
        assert_eq!(doc.bytes_de_variable(&var), &bytes[..bytes.len() - 2]);
        assert_eq!(doc.fin_de_linea(), "\r\n");
    }

    #[test]
    fn variables_de_primer_nivel_en_orden() {
        let doc = Documento::leer(b"A = 1\nB = {\n}\nlocal c = 2\nD, E = 1, 2\n").unwrap();
        let nombres: Vec<_> = doc.variables().into_iter().map(|v| v.nombre).collect();
        assert_eq!(nombres, ["A", "B"]);
    }

    #[test]
    fn un_fichero_que_no_parsea_no_es_documento() {
        assert!(Documento::leer(b"Var = {\n[\"a\"] = \n").is_none());
    }

    #[test]
    fn incluir_solo_deja_lo_nombrado() {
        let fuente = "Db = {\n[\"a\"] = 1,\n[\"b\"] = {\n[\"x\"] = true,\n[\"y\"] = false,\n},\n[\"c\"] = 3,\n}\n";
        let incluir = rutas(&["a", "b/y"]);
        assert_eq!(
            recorte(fuente, "Db", &seleccion(Some(&incluir), &[], &[])).unwrap(),
            "{\n[\"a\"] = 1,\n[\"b\"] = {\n[\"y\"] = false,\n},\n}"
        );
        let nada = rutas(&["zz"]);
        assert_eq!(
            recorte(fuente, "Db", &seleccion(Some(&nada), &[], &[])),
            None
        );
    }

    #[test]
    fn entera_copia_el_valor_tal_cual() {
        let fuente = "Db = {\n  a = 1, -- comentario\n}\n";
        assert_eq!(
            recorte(fuente, "Db", &seleccion(None, &[], &[])).unwrap(),
            "{\n  a = 1, -- comentario\n}"
        );
    }

    #[test]
    fn excluir_quita_lo_nombrado_a_cualquier_nivel() {
        let fuente = "Db = {\n[\"a\"] = 1,\n[\"b\"] = {\n[\"x\"] = 1,\n[\"y\"] = 2,\n},\n}\n";
        let excluir = rutas(&["a", "b/x"]);
        assert_eq!(
            recorte(fuente, "Db", &seleccion(None, &excluir, &[])).unwrap(),
            "{\n[\"b\"] = {\n[\"y\"] = 2,\n},\n}"
        );
    }

    #[test]
    fn solo_personaje_deja_su_entrada_con_el_marcador() {
        let fuente = concat!(
            "Db = {\n[\"Scales\"] = {\n[\"S1\"] = {\n[\"Value\"] = 1,\n[\"PerCharacterOptions\"] = {\n",
            "[\"Goldberg Joestar\"] = {\n[\"Visible\"] = true,\n},\n[\"Otro Pj\"] = {\n[\"Visible\"] = false,\n},\n",
            "},\n},\n},\n[\"Debug\"] = false,\n}\n"
        );
        let incluir = rutas(&["Scales"]);
        let solo = rutas(&["Scales/*/PerCharacterOptions/$PJ"]);
        let salida = recorte(fuente, "Db", &seleccion(Some(&incluir), &[], &solo)).unwrap();
        assert!(salida.contains("[\"@@WOWPROFILE:PERSONAJE@@\"] = {\n[\"Visible\"] = true,\n}"));
        assert!(!salida.contains("Otro Pj"));
        assert!(!salida.contains("Goldberg"));
        assert!(salida.contains("[\"Value\"] = 1"));
        assert!(!salida.contains("Debug"));
    }

    #[test]
    fn listas_sin_clave_solo_viajan_enteras() {
        let fuente =
            "Db = {\n[\"l\"] = {\n\"a\",\n\"b\",\n},\n[\"m\"] = {\n\"c\",\n[\"k\"] = 1,\n},\n}\n";
        let incluir = rutas(&["l", "m/k"]);
        assert_eq!(
            recorte(fuente, "Db", &seleccion(Some(&incluir), &[], &[])).unwrap(),
            "{\n[\"l\"] = {\n\"a\",\n\"b\",\n},\n[\"m\"] = {\n[\"k\"] = 1,\n},\n}"
        );
    }

    #[test]
    fn solo_datos_admite_datos_y_rechaza_codigo() {
        let datos = "A = {
[\"s\"] = \"x\",
[1] = -2.5,
n = nil,
{ true, false },
}
B = 3
";
        assert!(Documento::leer(datos.as_bytes()).unwrap().solo_datos());
        for codigo in [
            "A = RunScript(\"x\")",
            "A = { k = os.exit() }",
            "A = { k = Otra }",
            "A = function() end",
            "A = 1 + 2",
            "A = -x",
            "A = ...",
            "RunScript(\"x\")",
            "local A = 1",
            "A, B = 1, 2",
            "A = 1
return A",
            "A = { [f()] = 1 }",
        ] {
            let documento = Documento::leer(codigo.as_bytes()).unwrap();
            assert!(!documento.solo_datos(), "{codigo}");
        }
    }

    #[test]
    fn valor_suelto_es_exactamente_una_tabla() {
        let documento = Documento::leer_valor(
            b"{
[\"a\"] = 1,
}
",
        )
        .unwrap();
        let valor = documento.valor_suelto().unwrap();
        assert_eq!(
            documento.bytes_de(valor.valor),
            b"{
[\"a\"] = 1,
}"
        );
        assert!(documento.solo_datos());
        assert!(Documento::leer_valor(b"{} X = 1").is_none());
        assert!(Documento::leer_valor(b"{} return").is_none());
        assert!(Documento::leer_valor(b"{").is_none());
    }

    #[test]
    fn con_variable_cambia_solo_su_valor_o_la_anade() {
        let mut bytes = b"-- x
A = {
[\"k\"] = \""
            .to_vec();
        bytes.push(0xFF);
        bytes.extend(
            b"\",
}
B = 1
",
        );
        let documento = Documento::leer(&bytes).unwrap();
        let mut esperado = b"-- x
A = {
[\"k\"] = \""
            .to_vec();
        esperado.push(0xFF);
        esperado.extend(
            b"\",
}
B = 2
",
        );
        assert_eq!(documento.con_variable("B", b"2"), esperado);
        let mut anadida = bytes.clone();
        anadida.extend(
            b"C = 3
",
        );
        assert_eq!(documento.con_variable("C", b"3"), anadida);
        let sin_salto = Documento::leer(b"A = 1").unwrap();
        assert_eq!(
            sin_salto.con_variable("C", b"3"),
            b"A = 1
C = 3
"
        );
    }

    #[test]
    fn literal_escapa_comillas_y_barras() {
        assert_eq!(literal_latin1("Gø \"x\" \\"), a_latin1("Gø \\\"x\\\" \\\\"));
        assert_eq!(literal_latin1("a\nb\r"), "a\\010b\\013");
        // Escapado, vuelve a parsear.
        let texto = format!("A = \"{}\"", literal_latin1("x\n\"\\y"));
        assert!(Documento::leer(texto.as_bytes()).unwrap().solo_datos());
    }

    #[test]
    fn campo_y_claves_de_cadena() {
        let doc = Documento::leer(
            "Db = {\n[\"pk\"] = {\n[\"Gøld Joe\"] = \"Default\",\n},\nn = 1,\n}\n".as_bytes(),
        )
        .unwrap();
        let var = doc.variable("Db").unwrap();
        let t = tabla(var.valor).unwrap();
        assert!(campo(t, "n").is_some());
        let pk = tabla(campo(t, "pk").unwrap()).unwrap();
        let (clave, valor) = campos(pk).next().unwrap();
        assert_eq!(a_utf8(&clave).unwrap(), "Gøld Joe");
        assert_eq!(cadena(valor).unwrap(), "Default");
        assert_eq!(claves_de_cadena(var.valor), ["pk", "Gøld Joe"]);
    }
}
