//! Modo Edición de Blizzard (#71): el adaptador interno `modo-edicion` (`docs/adapters.md`). Lógica pura
//! sobre el texto de los dos ficheros; quien lee y escribe en disco es Exportación e Importación.
//!
//! Medido en WoW Forever 1.60.1 (comentario en #71):
//! - Los dos ficheros son una línea de valores separados por espacios que termina en un byte nulo.
//! - `edit-mode-cache-account.txt` (cuenta): cabecera `5 <n>` y `n` números; después, los diseños propios uno detrás
//!   de otro, cada uno completo: `<largo del nombre> <nombre> <tipo> <nº de marcos>` y 10 valores por marco. El largo
//!   es en bytes UTF-8 (`Diseño ñ` → `10`) y el juego corta el nombre por él.
//! - `edit-mode-cache-character.txt` (personaje): `<n>` y `2n` valores; el primero es el índice del diseño activo.
//!   0–2 son los de Blizzard; los propios empiezan en 3, en el orden del fichero de cuenta.
//!
//! Nada se reescribe: sacar un diseño copia sus bytes tal cual y añadir uno deja intacto lo que ya había.

use std::fmt;
use std::ops::Range;

/// Nombre del adaptador interno en el manifiesto (`docs/adapters.md`).
pub const ADAPTADOR: &str = "modo-edicion";
pub const VERSION_ADAPTADOR: u32 = 1;
/// El fichero de la cuenta: los diseños propios.
pub const FICHERO_CUENTA: &str = "edit-mode-cache-account.txt";
/// El fichero del personaje: dice qué diseño usa. No viaja; se recalcula en el destino.
pub const FICHERO_PERSONAJE: &str = "edit-mode-cache-character.txt";
/// Su ruta lógica, como las de las piezas.
pub const DESTINO_PERSONAJE: &str = "personaje/edit-mode-cache-character.txt";
/// Índice del primer diseño propio en el fichero del personaje: 0–2 son los de Blizzard.
pub const PRIMER_PROPIO: usize = 3;
/// Versión del formato medida. Otra versión no se toca.
const VERSION: &str = "5";
/// Valores de cada marco de un diseño.
const VALORES_POR_MARCO: usize = 10;
/// Ranuras de especialización del fichero del personaje que hay cuando no existe y se crea.
const RANURAS_NUEVAS: usize = 5;
const SIN_DISENO: &str = "-1";
/// Lo último de los dos ficheros.
const FIN: char = '\0';

/// Por qué un fichero del Modo Edición no se entiende. Si no se entiende, no se toca.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatoNoValido {
    NoEsTexto,
    Version,
    Cabecera,
    Diseno { posicion: usize },
    Personaje,
    IndiceFueraDeRango { indice: usize, disenos: usize },
}

impl fmt::Display for FormatoNoValido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoEsTexto => write!(f, "no es texto UTF-8"),
            Self::Version => write!(f, "versión del Modo Edición distinta de {VERSION}"),
            Self::Cabecera => write!(f, "cabecera del fichero de cuenta incompleta"),
            Self::Diseno { posicion } => write!(f, "el diseño {posicion} no está completo"),
            Self::Personaje => write!(f, "fichero del personaje incompleto"),
            Self::IndiceFueraDeRango { indice, disenos } => {
                write!(
                    f,
                    "el personaje usa el diseño {indice} y la cuenta solo tiene {disenos}"
                )
            }
        }
    }
}

/// Un diseño propio dentro del fichero de cuenta: su nombre y dónde están sus bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diseno {
    pub nombre: String,
    pub rango: Range<usize>,
}

/// El fichero de cuenta leído: la cabecera y los diseños, como rangos sobre el texto original.
#[derive(Debug)]
pub struct Cuenta<'a> {
    texto: &'a str,
    cabecera: Range<usize>,
    pub disenos: Vec<Diseno>,
}

impl<'a> Cuenta<'a> {
    pub fn leer(texto: &'a str) -> Result<Self, FormatoNoValido> {
        let texto = sin_fin(texto);
        let mut lector = Lector::nuevo(texto);
        let version = lector.token().ok_or(FormatoNoValido::Cabecera)?;
        if &texto[version.clone()] != VERSION {
            return Err(FormatoNoValido::Version);
        }
        let n = lector.numero().ok_or(FormatoNoValido::Cabecera)?;
        lector.saltar(n).ok_or(FormatoNoValido::Cabecera)?;
        let cabecera = version.start..lector.fin_ultimo;
        let mut disenos = Vec::new();
        while lector.queda_algo() {
            let posicion = disenos.len();
            disenos.push(
                lector
                    .diseno()
                    .ok_or(FormatoNoValido::Diseno { posicion })?,
            );
        }
        Ok(Self {
            texto,
            cabecera,
            disenos,
        })
    }

    pub fn cabecera(&self) -> &'a str {
        &self.texto[self.cabecera.clone()]
    }

    pub fn texto_de(&self, diseno: &Diseno) -> &'a str {
        &self.texto[diseno.rango.clone()]
    }

    fn tiene_nombre(&self, nombre: &str) -> bool {
        self.disenos.iter().any(|d| d.nombre == nombre)
    }
}

/// Índice del diseño activo de un personaje: el primer valor tras el número de ranuras.
pub fn indice_activo(personaje: &str) -> Result<Option<usize>, FormatoNoValido> {
    let valores = valores_personaje(personaje)?;
    // `-1` (u otro negativo): sin diseño.
    Ok(valores[0].parse::<usize>().ok())
}

/// El diseño propio que usa el personaje, si usa uno. `None` si usa uno de Blizzard o ninguno.
pub fn diseno_activo<'a>(
    cuenta: &Cuenta<'a>,
    personaje: &str,
) -> Result<Option<Diseno>, FormatoNoValido> {
    let Some(indice) = indice_activo(personaje)? else {
        return Ok(None);
    };
    let Some(posicion) = indice.checked_sub(PRIMER_PROPIO) else {
        return Ok(None);
    };
    cuenta
        .disenos
        .get(posicion)
        .cloned()
        .map(Some)
        .ok_or(FormatoNoValido::IndiceFueraDeRango {
            indice,
            disenos: cuenta.disenos.len(),
        })
}

/// Un fichero de cuenta con un único diseño: así viaja un diseño en el paquete, y se valida con el mismo lector.
pub fn paquete_de(cuenta: &Cuenta<'_>, diseno: &Diseno) -> String {
    format!("{} {}{FIN}", cuenta.cabecera(), cuenta.texto_de(diseno))
}

/// Resultado de añadir un diseño al fichero de cuenta del destino.
#[derive(Debug, PartialEq, Eq)]
pub struct Anadido {
    pub texto: String,
    pub nombre: String,
    /// Índice que tiene que poner el personaje para usarlo.
    pub indice: usize,
    /// `false` si el destino ya tenía un diseño idéntico (con cualquier nombre): se usa ese y no se añade nada.
    pub nuevo: bool,
}

/// El diseño propio que usa el personaje, listo para el paquete: su nombre y un fichero de cuenta con solo él. `None`
/// si usa uno de Blizzard o ninguno.
pub fn exportar(
    cuenta: &[u8],
    personaje: &[u8],
) -> Result<Option<(String, Vec<u8>)>, FormatoNoValido> {
    let cuenta = Cuenta::leer(texto(cuenta)?)?;
    let activo = diseno_activo(&cuenta, texto(personaje)?)?;
    Ok(activo.map(|d| (d.nombre.clone(), paquete_de(&cuenta, &d).into_bytes())))
}

/// Lo que queda en los dos ficheros del destino tras importar el diseño del paquete.
#[derive(Debug, PartialEq, Eq)]
pub struct Importado {
    pub cuenta: Vec<u8>,
    pub personaje: Vec<u8>,
    pub nombre: String,
    pub nuevo: bool,
}

/// Añade el diseño del paquete a la cuenta del destino (o reutiliza uno idéntico) y pone al personaje a usarlo.
pub fn importar(
    cuenta: Option<&[u8]>,
    personaje: Option<&[u8]>,
    paquete: &[u8],
) -> Result<Importado, FormatoNoValido> {
    let anadido = anadir(cuenta.map(texto).transpose()?, texto(paquete)?)?;
    let personaje = con_indice(personaje.map(texto).transpose()?, anadido.indice)?;
    Ok(Importado {
        cuenta: anadido.texto.into_bytes(),
        personaje: personaje.into_bytes(),
        nombre: anadido.nombre,
        nuevo: anadido.nuevo,
    })
}

fn texto(bytes: &[u8]) -> Result<&str, FormatoNoValido> {
    std::str::from_utf8(bytes).map_err(|_| FormatoNoValido::NoEsTexto)
}

/// Añade a `destino` (o a un fichero nuevo con la cabecera del paquete, si no hay) el único diseño de `paquete`. Lo que
/// ya había no cambia ni un byte; si el nombre ya existe, el nuevo se llama «<nombre> 2», «<nombre> 3»…
pub fn anadir(destino: Option<&str>, paquete: &str) -> Result<Anadido, FormatoNoValido> {
    let origen = Cuenta::leer(paquete)?;
    let [diseno] = origen.disenos.as_slice() else {
        return Err(FormatoNoValido::Diseno { posicion: 0 });
    };
    let base = destino.map_or(origen.cabecera(), sin_fin).trim_end();
    let cuenta = Cuenta::leer(base)?;
    let cuerpo = sin_nombre(origen.texto_de(diseno), &diseno.nombre);
    if let Some((posicion, igual)) = cuenta
        .disenos
        .iter()
        .enumerate()
        .find(|(_, d)| sin_nombre(cuenta.texto_de(d), &d.nombre) == cuerpo)
    {
        // El destino queda tal cual, byte a byte: no se añade nada.
        return Ok(Anadido {
            texto: destino.map_or_else(|| format!("{base}{FIN}"), str::to_owned),
            nombre: igual.nombre.clone(),
            indice: PRIMER_PROPIO + posicion,
            nuevo: false,
        });
    }
    let nombre = nombre_libre(&cuenta, &diseno.nombre);
    let texto = format!("{base} {} {nombre} {cuerpo}{FIN}", largo(&nombre));
    Ok(Anadido {
        texto,
        nombre,
        indice: PRIMER_PROPIO + cuenta.disenos.len(),
        nuevo: true,
    })
}

/// El fichero del personaje con `indice` como diseño activo; el resto de ranuras, como estaban. Si no había fichero,
/// todas las ranuras usan ese diseño.
pub fn con_indice(personaje: Option<&str>, indice: usize) -> Result<String, FormatoNoValido> {
    let Some(texto) = personaje else {
        let ranuras = vec![indice.to_string(); RANURAS_NUEVAS].join(" ");
        let vacias = [SIN_DISENO; RANURAS_NUEVAS].join(" ");
        return Ok(format!("{RANURAS_NUEVAS} {ranuras} {vacias}{FIN}"));
    };
    valores_personaje(texto)?;
    let mut lector = Lector::nuevo(texto);
    lector.token().ok_or(FormatoNoValido::Personaje)?;
    let activo = lector.token().ok_or(FormatoNoValido::Personaje)?;
    Ok(format!(
        "{}{indice}{}",
        &texto[..activo.start],
        &texto[activo.end..]
    ))
}

fn valores_personaje(texto: &str) -> Result<Vec<&str>, FormatoNoValido> {
    let mut tokens = sin_fin(texto).split_ascii_whitespace();
    let ranuras: usize = tokens
        .next()
        .and_then(|t| t.parse().ok())
        .filter(|n| *n > 0)
        .ok_or(FormatoNoValido::Personaje)?;
    let valores: Vec<&str> = tokens.collect();
    let completos = ranuras.checked_mul(2) == Some(valores.len())
        && valores.iter().all(|v| v.parse::<i64>().is_ok());
    if completos {
        Ok(valores)
    } else {
        Err(FormatoNoValido::Personaje)
    }
}

/// El texto sin el byte nulo final (si lo tiene).
fn sin_fin(texto: &str) -> &str {
    texto.strip_suffix(FIN).unwrap_or(texto)
}

/// Largo de un nombre tal como lo escribe el juego: bytes UTF-8 (medido con `Diseño ñ`).
fn largo(nombre: &str) -> usize {
    nombre.len()
}

fn nombre_libre(cuenta: &Cuenta<'_>, nombre: &str) -> String {
    if !cuenta.tiene_nombre(nombre) {
        return nombre.to_owned();
    }
    (2..)
        .map(|n| format!("{nombre} {n}"))
        .find(|candidato| !cuenta.tiene_nombre(candidato))
        .unwrap_or_else(|| nombre.to_owned())
}

/// El texto de un diseño sin `<largo> <nombre> ` delante: lo que sigue al nombre.
fn sin_nombre<'t>(diseno: &'t str, nombre: &str) -> &'t str {
    let prefijo = format!("{} {nombre} ", largo(nombre));
    diseno.strip_prefix(prefijo.as_str()).unwrap_or(diseno)
}

/// Recorre el texto por valores separados por espacios. Recuerda dónde acabó el último para poder cortar rangos.
struct Lector<'a> {
    texto: &'a str,
    pos: usize,
    fin_ultimo: usize,
}

impl<'a> Lector<'a> {
    fn nuevo(texto: &'a str) -> Self {
        Self {
            texto,
            pos: 0,
            fin_ultimo: 0,
        }
    }

    fn saltar_espacios(&mut self) {
        let resto = &self.texto[self.pos..];
        self.pos += resto.len() - resto.trim_start().len();
    }

    fn queda_algo(&mut self) -> bool {
        self.saltar_espacios();
        self.pos < self.texto.len()
    }

    fn token(&mut self) -> Option<Range<usize>> {
        self.saltar_espacios();
        let resto = &self.texto[self.pos..];
        let largo = resto.find(char::is_whitespace).unwrap_or(resto.len());
        if largo == 0 {
            return None;
        }
        let rango = self.pos..self.pos + largo;
        self.pos = rango.end;
        self.fin_ultimo = rango.end;
        Some(rango)
    }

    fn numero(&mut self) -> Option<usize> {
        let rango = self.token()?;
        self.texto[rango].parse().ok()
    }

    fn saltar(&mut self, n: usize) -> Option<()> {
        (0..n).try_for_each(|_| self.token().map(|_| ()))
    }

    /// `<largo> <nombre> <tipo> <nº de marcos>` y 10 valores por marco. El nombre se corta por su largo (puede llevar
    /// espacios) y tiene que acabar en un espacio; vacío o con caracteres de control no es un nombre.
    fn diseno(&mut self) -> Option<Diseno> {
        let inicio = self.token()?;
        let largo: usize = self.texto[inicio.clone()].parse().ok()?;
        let nombre_inicio = inicio.end.checked_add(1)?;
        let nombre_fin = nombre_inicio.checked_add(largo)?;
        let nombre = self.texto.get(nombre_inicio..nombre_fin)?;
        let valido = !nombre.is_empty() && !nombre.chars().any(char::is_control);
        if !valido || !self.texto[nombre_fin..].starts_with(' ') {
            return None;
        }
        self.pos = nombre_fin;
        let tipo = self.token()?;
        self.texto[tipo].parse::<i64>().ok()?;
        let marcos = self.numero()?;
        self.saltar(marcos.checked_mul(VALORES_POR_MARCO)?)?;
        Some(Diseno {
            nombre: nombre.to_owned(),
            rango: inicio.start..self.fin_ultimo,
        })
    }
}

#[cfg(test)]
mod tests;
