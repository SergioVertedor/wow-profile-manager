//! Leer un `.wowprofile` según «Leer un paquete de forma segura» (`docs/formato-wowprofile.md`), pasos 1–5. El paso 6
//! (cada `destino` a una ruta real) necesita el personaje destino y lo hace Importación. Nada de lo que trae el paquete
//! se usa como ruta: el contenido queda en memoria, por `id`. Las piezas de addon (formato 1.1) no: se comprueban de una
//! en una al leer y se sueltan; se vuelven a leer del fichero al instalarlas.

use super::{
    AmbitoEntrada, CARPETA_DATOS, Categoria, MANIFIESTO, MAX_BYTES_ADDONS, MAX_BYTES_PIEZA,
    MAX_BYTES_TOTAL, MAX_CLAVES_QUITADAS, MAX_DEPENDENCIAS, MAX_ENLACE, MAX_PIEZAS, MAX_TITULO,
    MAX_VERSION, NOMBRE_FORMATO, PREFIJO_DESTINO_ADDONS, Tipo, id_de_addon_valido,
};
use crate::addon_zip::{self, MAX_BYTES_ADDON};
use crate::error::{ErrorNucleo, MotivoPaquete};
use crate::operaciones::{sha256, sha256_de};
use serde::de::IntoDeserializer;
use serde::de::value::{Error as ErrorValor, StrDeserializer};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use zip::ZipArchive;

/// `manifest.json` como mucho (paso 2).
const MAX_BYTES_MANIFIESTO: u64 = 1024 * 1024;
/// El `.wowprofile` entero: las piezas comprimidas no pasan de su tamaño sin comprimir más un margen para el ZIP y el
/// manifiesto. Lo que pase de aquí no se lee.
const MAX_BYTES_FICHERO: u64 = MAX_BYTES_TOTAL as u64 + MAX_BYTES_ADDONS + 4 * MAX_BYTES_MANIFIESTO;
const MAYOR_ADMITIDA: &str = "1";
/// La versión menor más nueva que entiende esta app.
const MENOR_CONOCIDA: u32 = 1;
/// La que añade las piezas de addon.
const MENOR_CON_ADDONS: u32 = 1;
const MAX_NOMBRE_PERFIL: usize = 80;
const MAX_DESCRIPCION: usize = 500;
const MAX_ID_ENTRADA: usize = 64;
const MAX_TEXTO_ENTRADA: usize = 128;
const MAX_CLASE: usize = 20;
const MIN_CLASE: usize = 2;
const MAX_VARIABLE: usize = 128;
const PRODUCTOS: [&str; 2] = ["wowf", "wow"];
/// Las de la 1.0; la 1.1 añade `addon`. En una 1.x más nueva puede haber más: se aceptan y se marcan como desconocidas.
const CATEGORIAS_1_0: [&str; 10] = [
    "macros",
    "teclas",
    "teclas-clic",
    "opciones",
    "chat",
    "ventanas",
    "modo-edicion",
    "addons-activos",
    "config-addon",
    "marcos-banda",
];
const TIPOS_1_0: [&str; 4] = ["fichero", "fichero-filtrado", "perfil-acedb", "adaptador"];
const CATEGORIA_ADDON: &str = "addon";
const TIPO_CARPETA: &str = "carpeta";
const EXTENSIONES_DESTINO: [&str; 3] = ["txt", "wtf", "lua"];
const MARCADORES: [&str; 2] = ["PERSONAJE", "REINO"];
const TIPO_FICHERO_FILTRADO: &str = "fichero-filtrado";
const TIPO_PERFIL_ACEDB: &str = "perfil-acedb";
const TIPO_ADAPTADOR: &str = "adaptador";
const CATEGORIA_CONFIG_ADDON: &str = "config-addon";
/// Largo máximo de un texto del paquete en un mensaje de diagnóstico.
const MAX_DIAGNOSTICO: usize = 120;
/// Campos de cada objeto en el schema 1.0 y 1.1 (`additionalProperties: false`).
const CAMPOS_RAIZ: [&str; 5] = ["formato", "perfil", "origen", "addons", "entradas"];
const CAMPOS_FORMATO: [&str; 2] = ["nombre", "version"];
const CAMPOS_PERFIL: [&str; 3] = ["nombre", "descripcion", "creado"];
const CAMPOS_ORIGEN: [&str; 4] = ["producto", "build", "interfaz", "clase"];
const CAMPOS_ADDON: [&str; 5] = ["id", "titulo", "version", "enlace", "dependencias"];
const CAMPOS_ENTRADA: [&str; 13] = [
    "id",
    "categoria",
    "tipo",
    "ambito",
    "destino",
    "addon",
    "sha256",
    "bytes",
    "marcadores",
    "claves_quitadas",
    "variable",
    "perfil",
    "adaptador",
];
const CAMPOS_ADAPTADOR: [&str; 2] = ["id", "version"];
const PREFIJO_SAVEDVARIABLES: &str = "SavedVariables/";

/// Un paquete leído y comprobado entero: el manifiesto y el contenido de cada entrada, por `id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaqueteLeido {
    /// SHA-256 del fichero `.wowprofile`: lo identifica entre abrir y aplicar.
    pub sha256: String,
    pub manifiesto: ManifiestoLeido,
    piezas: HashMap<String, Vec<u8>>,
}

impl PaqueteLeido {
    pub fn pieza(&self, id: &str) -> Option<&[u8]> {
        self.piezas.get(id).map(Vec::as_slice)
    }

    /// Un paquete ya «leído» sin pasar por un ZIP, para probar lo que viene después de leerlo.
    #[cfg(test)]
    pub(crate) fn de_prueba(manifiesto: ManifiestoLeido, piezas: &[(&str, &[u8])]) -> Self {
        Self {
            sha256: "0".repeat(64),
            manifiesto,
            piezas: piezas
                .iter()
                .map(|(id, bytes)| ((*id).to_owned(), bytes.to_vec()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FormatoLeido {
    pub nombre: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PerfilLeido {
    pub nombre: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub descripcion: Option<String>,
    pub creado: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct OrigenLeido {
    pub producto: String,
    pub build: String,
    pub interfaz: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clase: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AddonLeido {
    pub id: String,
    pub titulo: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enlace: Option<String>,
    #[serde(default)]
    pub dependencias: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AdaptadorLeido {
    pub id: String,
    pub version: u32,
}

/// Una entrada tal como viene. `categoria` y `tipo` quedan como texto: en una 1.x más nueva puede haber valores que esta
/// app no conoce, y eso no rompe la lectura (se ofrecen desactivadas).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct EntradaLeida {
    pub id: String,
    pub categoria: String,
    pub tipo: String,
    pub ambito: AmbitoEntrada,
    pub destino: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addon: Option<String>,
    pub sha256: String,
    pub bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marcadores: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claves_quitadas: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perfil: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adaptador: Option<AdaptadorLeido>,
}

impl EntradaLeida {
    /// La categoría si esta app la conoce.
    pub fn categoria_conocida(&self) -> Option<Categoria> {
        de_texto(&self.categoria)
    }

    /// El tipo si esta app lo conoce.
    pub fn tipo_conocido(&self) -> Option<Tipo> {
        de_texto(&self.tipo)
    }
}

/// Un valor de un `enum` de serde a partir de su nombre en el formato.
fn de_texto<T: for<'de> Deserialize<'de>>(texto: &str) -> Option<T> {
    let valor: StrDeserializer<'_, ErrorValor> = texto.into_deserializer();
    T::deserialize(valor).ok()
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ManifiestoLeido {
    pub formato: FormatoLeido,
    pub perfil: PerfilLeido,
    pub origen: OrigenLeido,
    pub addons: Vec<AddonLeido>,
    pub entradas: Vec<EntradaLeida>,
}

fn invalido(motivo: MotivoPaquete, detalle: impl Into<String>) -> ErrorNucleo {
    ErrorNucleo::PaqueteInvalido {
        motivo,
        detalle: detalle.into(),
    }
}

/// Lee el fichero sin pasar del tamaño máximo y lo comprueba entero, sin cargarlo en memoria: con addons puede ocupar
/// cientos de MiB.
pub fn leer_fichero(ruta: &Path) -> Result<PaqueteLeido, ErrorNucleo> {
    let lectura = |e: std::io::Error| ErrorNucleo::lectura(ruta, &e);
    let mut fichero = std::fs::File::open(ruta).map_err(lectura)?;
    if fichero.metadata().map_err(lectura)?.len() > MAX_BYTES_FICHERO {
        return Err(invalido(MotivoPaquete::DemasiadoGrande, ""));
    }
    // El fichero puede crecer mientras se calcula el hash: si pasa del máximo, no se lee.
    let hash =
        sha256_de(BufReader::new((&fichero).take(MAX_BYTES_FICHERO + 1))).map_err(lectura)?;
    if fichero.stream_position().map_err(lectura)? > MAX_BYTES_FICHERO {
        return Err(invalido(MotivoPaquete::DemasiadoGrande, ""));
    }
    fichero.seek(SeekFrom::Start(0)).map_err(lectura)?;
    leer_zip(BufReader::new(fichero), hash, MAX_BYTES_TOTAL as u64)
}

/// Pasos 1–5 sobre el fichero en memoria. Si algo falla no hay paquete: nunca uno «medio válido».
#[cfg(test)]
pub fn leer(bytes: &[u8]) -> Result<PaqueteLeido, ErrorNucleo> {
    leer_con_tope(bytes, MAX_BYTES_TOTAL as u64)
}

#[cfg(test)]
fn leer_con_tope(bytes: &[u8], maximo_total: u64) -> Result<PaqueteLeido, ErrorNucleo> {
    leer_zip(std::io::Cursor::new(bytes), sha256(bytes), maximo_total)
}

fn leer_zip<R: Read + Seek>(
    lector: R,
    sha256: String,
    maximo_total: u64,
) -> Result<PaqueteLeido, ErrorNucleo> {
    let mut zip =
        ZipArchive::new(lector).map_err(|e| invalido(MotivoPaquete::NoEsZip, e.to_string()))?;
    // Paso 1: antes de leer nada más.
    if zip.len() > MAX_PIEZAS + 1 {
        return Err(invalido(
            MotivoPaquete::DemasiadasEntradas,
            zip.len().to_string(),
        ));
    }
    let manifiesto = leer_manifiesto(&mut zip)?;
    comprobar_nombres(&mut zip, &manifiesto)?;
    let piezas = leer_piezas(&mut zip, &manifiesto, maximo_total)?;
    Ok(PaqueteLeido {
        sha256,
        manifiesto,
        piezas,
    })
}

/// Paso 2: como mucho 1 MiB, versión mayor 1 y lo que pide el schema.
fn leer_manifiesto<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
) -> Result<ManifiestoLeido, ErrorNucleo> {
    let fichero = zip
        .by_name(MANIFIESTO)
        .map_err(|e| invalido(MotivoPaquete::ManifiestoIlegible, e.to_string()))?;
    let mut bytes = Vec::new();
    fichero
        .take(MAX_BYTES_MANIFIESTO + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| invalido(MotivoPaquete::ManifiestoIlegible, e.to_string()))?;
    if bytes.len() as u64 > MAX_BYTES_MANIFIESTO {
        return Err(invalido(MotivoPaquete::ManifiestoIlegible, MANIFIESTO));
    }
    let valor: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| invalido(MotivoPaquete::ManifiestoIlegible, e.to_string()))?;
    let menor = version_admitida(&valor)?;
    // El schema no admite `null` en ningún campo; `serde` lo tomaría por «no viene».
    if !sin_nulos(&valor) {
        return Err(invalido(MotivoPaquete::ManifiestoNoValido, "null"));
    }
    // En las versiones que conoce, nada que el schema no nombre. En una 1.x más nueva se ignora
    // (`docs/formato-wowprofile.md`).
    if menor <= MENOR_CONOCIDA {
        campos_conocidos(&valor)
            .map_err(|campo| invalido(MotivoPaquete::ManifiestoNoValido, campo))?;
    }
    let manifiesto: ManifiestoLeido = serde_json::from_value(valor)
        .map_err(|e| invalido(MotivoPaquete::ManifiestoNoValido, e.to_string()))?;
    validar(&manifiesto, menor)
        .map_err(|campo| invalido(MotivoPaquete::ManifiestoNoValido, campo))?;
    Ok(manifiesto)
}

/// La versión menor si la mayor es la que entiende esta app. Otra mayor pide actualizar la app.
fn version_admitida(valor: &serde_json::Value) -> Result<u32, ErrorNucleo> {
    let formato = &valor["formato"];
    let no_valido = || invalido(MotivoPaquete::ManifiestoNoValido, "formato");
    if formato["nombre"].as_str() != Some(NOMBRE_FORMATO) {
        return Err(no_valido());
    }
    let version = formato["version"].as_str().ok_or_else(no_valido)?;
    let (mayor, menor) = version.split_once('.').ok_or_else(no_valido)?;
    if !numero_sin_ceros(mayor) || !numero_sin_ceros(menor) {
        return Err(no_valido());
    }
    if mayor != MAYOR_ADMITIDA {
        return Err(ErrorNucleo::VersionPaquete {
            version: version.to_owned(),
        });
    }
    menor.parse().map_err(|_| no_valido())
}

fn sin_nulos(valor: &serde_json::Value) -> bool {
    match valor {
        serde_json::Value::Null => false,
        serde_json::Value::Array(lista) => lista.iter().all(sin_nulos),
        serde_json::Value::Object(mapa) => mapa.values().all(sin_nulos),
        _ => true,
    }
}

/// `additionalProperties: false` del schema 1.0 y 1.1, objeto a objeto. Devuelve el primer campo que sobra.
fn campos_conocidos(valor: &serde_json::Value) -> Result<(), String> {
    solo_campos(valor, &CAMPOS_RAIZ, "manifiesto")?;
    solo_campos(&valor["formato"], &CAMPOS_FORMATO, "formato")?;
    solo_campos(&valor["perfil"], &CAMPOS_PERFIL, "perfil")?;
    solo_campos(&valor["origen"], &CAMPOS_ORIGEN, "origen")?;
    let lista = |campo: &str| valor[campo].as_array().cloned().unwrap_or_default();
    for (i, addon) in lista("addons").iter().enumerate() {
        solo_campos(addon, &CAMPOS_ADDON, &format!("addons[{i}]"))?;
    }
    for (i, entrada) in lista("entradas").iter().enumerate() {
        solo_campos(entrada, &CAMPOS_ENTRADA, &format!("entradas[{i}]"))?;
        solo_campos(
            &entrada["adaptador"],
            &CAMPOS_ADAPTADOR,
            &format!("entradas[{i}].adaptador"),
        )?;
    }
    Ok(())
}

/// Si es un objeto, que no tenga campos fuera de `campos`. Lo que no es un objeto lo rechaza después `serde`.
fn solo_campos(valor: &serde_json::Value, campos: &[&str], donde: &str) -> Result<(), String> {
    let sobra = valor
        .as_object()
        .and_then(|mapa| mapa.keys().find(|k| !campos.contains(&k.as_str())));
    match sobra {
        Some(campo) => Err(format!("{donde}.{}", recortar(campo))),
        None => Ok(()),
    }
}

/// Un texto del paquete que va a un mensaje de diagnóstico, sin pasar de un largo razonable.
fn recortar(texto: &str) -> String {
    texto.chars().take(MAX_DIAGNOSTICO).collect()
}

/// `0` o un número sin ceros delante.
fn numero_sin_ceros(texto: &str) -> bool {
    !texto.is_empty()
        && texto.bytes().all(|b| b.is_ascii_digit())
        && (texto == "0" || !texto.starts_with('0'))
}

/// Lo que el schema 1.x exige y `serde` no comprueba. Devuelve el campo que falla.
fn validar(m: &ManifiestoLeido, menor: u32) -> Result<(), String> {
    validar_perfil(&m.perfil)?;
    validar_origen(&m.origen)?;
    if m.addons.len() > MAX_PIEZAS {
        return Err("addons".into());
    }
    for (i, addon) in m.addons.iter().enumerate() {
        validar_addon(addon).map_err(|campo| format!("addons[{i}].{campo}"))?;
    }
    if m.entradas.len() > MAX_PIEZAS {
        return Err("entradas".into());
    }
    let mut ids = HashSet::new();
    for (i, entrada) in m.entradas.iter().enumerate() {
        validar_entrada(entrada, menor).map_err(|campo| format!("entradas[{i}].{campo}"))?;
        if !ids.insert(entrada.id.as_str()) {
            return Err(format!("entradas[{i}].id repetido"));
        }
    }
    Ok(())
}

fn exigir(cumple: bool, campo: &str) -> Result<(), String> {
    if cumple {
        Ok(())
    } else {
        Err(campo.to_owned())
    }
}

fn largo_entre(texto: &str, minimo: usize, maximo: usize) -> bool {
    (minimo..=maximo).contains(&texto.chars().count())
}

fn validar_perfil(perfil: &PerfilLeido) -> Result<(), String> {
    exigir(
        largo_entre(&perfil.nombre, 1, MAX_NOMBRE_PERFIL),
        "perfil.nombre",
    )?;
    let descripcion = perfil.descripcion.as_deref().unwrap_or_default();
    exigir(
        largo_entre(descripcion, 0, MAX_DESCRIPCION),
        "perfil.descripcion",
    )?;
    exigir(fecha_utc(&perfil.creado), "perfil.creado")
}

fn validar_origen(origen: &OrigenLeido) -> Result<(), String> {
    exigir(
        PRODUCTOS.contains(&origen.producto.as_str()),
        "origen.producto",
    )?;
    let partes: Vec<_> = origen.build.split('.').collect();
    let build = partes.len() == 4
        && partes
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    exigir(build, "origen.build")?;
    exigir(origen.interfaz >= 1, "origen.interfaz")?;
    let clase = origen.clase.as_deref().is_none_or(|c| {
        largo_entre(c, MIN_CLASE, MAX_CLASE) && c.bytes().all(|b| b.is_ascii_uppercase())
    });
    exigir(clase, "origen.clase")
}

fn validar_addon(addon: &AddonLeido) -> Result<(), String> {
    exigir(id_de_addon_valido(&addon.id), "id")?;
    exigir(largo_entre(&addon.titulo, 1, MAX_TITULO), "titulo")?;
    let version = addon.version.as_deref().unwrap_or_default();
    exigir(largo_entre(version, 0, MAX_VERSION), "version")?;
    let enlace = addon
        .enlace
        .as_deref()
        .is_none_or(|e| e.starts_with("https://") && largo_entre(e, 1, MAX_ENLACE));
    exigir(enlace, "enlace")?;
    exigir(addon.dependencias.len() <= MAX_DEPENDENCIAS, "dependencias")?;
    exigir(
        addon.dependencias.iter().all(|d| id_de_addon_valido(d)),
        "dependencias",
    )
}

fn validar_entrada(entrada: &EntradaLeida, menor: u32) -> Result<(), String> {
    exigir(id_de_entrada_valido(&entrada.id), "id")?;
    let mas_nueva = menor > MENOR_CONOCIDA;
    let con_addons = menor >= MENOR_CON_ADDONS;
    let categoria = entrada.categoria.as_str();
    let tipo = entrada.tipo.as_str();
    let categoria_conocida =
        CATEGORIAS_1_0.contains(&categoria) || (con_addons && categoria == CATEGORIA_ADDON);
    let tipo_conocido = TIPOS_1_0.contains(&tipo) || (con_addons && tipo == TIPO_CARPETA);
    exigir(mas_nueva || categoria_conocida, "categoria")?;
    exigir(mas_nueva || tipo_conocido, "tipo")?;
    exigir(
        entrada.addon.as_deref().is_none_or(id_de_addon_valido),
        "addon",
    )?;
    exigir(hash_valido(&entrada.sha256), "sha256")?;
    if es_de_addon(entrada) {
        return validar_carpeta(entrada);
    }
    exigir(tipo != TIPO_CARPETA, "tipo")?;
    exigir(entrada.ambito != AmbitoEntrada::Instalacion, "ambito")?;
    exigir(destino_valido(&entrada.destino), "destino")?;
    exigir(entrada.bytes <= MAX_BYTES_PIEZA as u64, "bytes")?;
    validar_opcionales(entrada)?;
    validar_condiciones(entrada)
}

/// La pieza es la carpeta de un addon (formato 1.1).
fn es_de_addon(entrada: &EntradaLeida) -> bool {
    entrada.categoria == CATEGORIA_ADDON
}

/// Una pieza de addon: tipo `carpeta`, de la instalación, con destino `addons/<addon>` y nada más.
fn validar_carpeta(entrada: &EntradaLeida) -> Result<(), String> {
    exigir(entrada.tipo == TIPO_CARPETA, "tipo")?;
    exigir(entrada.ambito == AmbitoEntrada::Instalacion, "ambito")?;
    let destino = entrada
        .addon
        .as_deref()
        .map(|addon| format!("{PREFIJO_DESTINO_ADDONS}{addon}"));
    exigir(
        destino.as_deref() == Some(entrada.destino.as_str()),
        "destino",
    )?;
    exigir(entrada.bytes <= MAX_BYTES_ADDON, "bytes")?;
    let sin_extras = entrada.marcadores.is_none()
        && entrada.claves_quitadas.is_none()
        && entrada.variable.is_none()
        && entrada.perfil.is_none()
        && entrada.adaptador.is_none();
    exigir(sin_extras, "addon")
}

fn validar_opcionales(entrada: &EntradaLeida) -> Result<(), String> {
    let claves = entrada.claves_quitadas.as_deref().unwrap_or_default();
    exigir(claves.len() <= MAX_CLAVES_QUITADAS, "claves_quitadas")?;
    exigir(
        claves.iter().all(|c| largo_entre(c, 1, MAX_TEXTO_ENTRADA)),
        "claves_quitadas",
    )?;
    exigir(
        entrada.variable.as_deref().is_none_or(variable_valida),
        "variable",
    )?;
    let perfil = entrada
        .perfil
        .as_deref()
        .is_none_or(|p| largo_entre(p, 1, MAX_TEXTO_ENTRADA));
    exigir(perfil, "perfil")?;
    let adaptador = entrada
        .adaptador
        .as_ref()
        .is_none_or(|a| id_de_entrada_valido(&a.id) && a.version >= 1);
    exigir(adaptador, "adaptador")?;
    let marcadores = entrada.marcadores.as_deref().unwrap_or_default();
    let distintos: HashSet<_> = marcadores.iter().collect();
    exigir(
        distintos.len() == marcadores.len()
            && marcadores.iter().all(|m| MARCADORES.contains(&m.as_str())),
        "marcadores",
    )
}

/// Los `if`/`then` del schema.
fn validar_condiciones(entrada: &EntradaLeida) -> Result<(), String> {
    let tipo = entrada.tipo.as_str();
    let config_addon = entrada.categoria == CATEGORIA_CONFIG_ADDON;
    exigir(
        tipo != TIPO_FICHERO_FILTRADO || entrada.claves_quitadas.is_some(),
        "claves_quitadas",
    )?;
    let acedb = tipo != TIPO_PERFIL_ACEDB
        || (entrada.addon.is_some()
            && entrada.variable.is_some()
            && entrada.perfil.is_some()
            && entrada.ambito == AmbitoEntrada::Cuenta
            && config_addon);
    exigir(acedb, "tipo")?;
    exigir(
        tipo != TIPO_ADAPTADOR || entrada.adaptador.is_some(),
        "adaptador",
    )?;
    exigir(!config_addon || entrada.addon.is_some(), "addon")
}

/// `^[a-z0-9][a-z0-9-]{0,63}$`.
pub(super) fn id_de_entrada_valido(id: &str) -> bool {
    let mut bytes = id.bytes();
    let primero = bytes
        .next()
        .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    primero
        && id.len() <= MAX_ID_ENTRADA
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// `^[0-9a-f]{64}$`.
fn hash_valido(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// `^[A-Za-z_][A-Za-z0-9_]{0,127}$`.
fn variable_valida(variable: &str) -> bool {
    let mut bytes = variable.bytes();
    let primero = bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_');
    primero
        && variable.len() <= MAX_VARIABLE
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// `^(personaje|cuenta)/(SavedVariables/)?[A-Za-z0-9_!\-]+(\.[A-Za-z0-9_!\-]+)*\.(txt|wtf|lua)$`.
pub(crate) fn destino_valido(destino: &str) -> bool {
    let Some(resto) = destino
        .strip_prefix("personaje/")
        .or_else(|| destino.strip_prefix("cuenta/"))
    else {
        return false;
    };
    let nombre = resto.strip_prefix(PREFIJO_SAVEDVARIABLES).unwrap_or(resto);
    let partes: Vec<_> = nombre.split('.').collect();
    let segmento = |s: &&str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'!' | b'-'))
    };
    partes.len() >= 2
        && partes.iter().all(segmento)
        && partes
            .last()
            .is_some_and(|e| EXTENSIONES_DESTINO.contains(e))
}

/// RFC 3339 en UTC: `AAAA-MM-DDTHH:MM:SS`, fracción opcional y `Z`.
fn fecha_utc(texto: &str) -> bool {
    let Some(sin_z) = texto.strip_suffix('Z') else {
        return false;
    };
    let (base, fraccion) = sin_z.split_once('.').unwrap_or((sin_z, "0"));
    let forma = base.len() == 19
        && base.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            10 => b == b'T',
            13 | 16 => b == b':',
            _ => b.is_ascii_digit(),
        });
    let campo =
        |desde: usize, hasta: usize| base.get(desde..hasta).and_then(|t| t.parse::<u32>().ok());
    let dia_valido = match (campo(0, 4), campo(5, 7), campo(8, 10)) {
        (Some(anio), Some(mes), Some(dia)) => (1..=dias_del_mes(anio, mes)).contains(&dia),
        _ => false,
    };
    let en_rango = forma
        && dia_valido
        && campo(11, 13).is_some_and(|h| h <= 23)
        && campo(14, 16).is_some_and(|m| m <= 59)
        && campo(17, 19).is_some_and(|s| s <= 60);
    en_rango && !fraccion.is_empty() && fraccion.bytes().all(|b| b.is_ascii_digit())
}

/// Días de ese mes (gregoriano); 0 si el mes no existe.
fn dias_del_mes(anio: u32, mes: u32) -> u32 {
    let bisiesto =
        (anio.is_multiple_of(4) && !anio.is_multiple_of(100)) || anio.is_multiple_of(400);
    match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if bisiesto => 29,
        2 => 28,
        _ => 0,
    }
}

/// Paso 3: exactamente `manifest.json` y un `datos/<id>` por entrada. Ni de más, ni de menos, ni repetidos.
fn comprobar_nombres<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    manifiesto: &ManifiestoLeido,
) -> Result<(), ErrorNucleo> {
    let mut esperados: HashSet<String> = manifiesto
        .entradas
        .iter()
        .map(|e| format!("{CARPETA_DATOS}{}", e.id))
        .collect();
    esperados.insert(MANIFIESTO.to_owned());
    let mut vistos = HashSet::new();
    for i in 0..zip.len() {
        let fichero = zip
            .by_index_raw(i)
            .map_err(|e| invalido(MotivoPaquete::EntradasDelZip, e.to_string()))?;
        let nombre = fichero
            .name()
            .map_err(|e| invalido(MotivoPaquete::EntradasDelZip, e.to_string()))?
            .into_owned();
        if !esperados.contains(&nombre) || !vistos.insert(nombre.clone()) {
            return Err(invalido(MotivoPaquete::EntradasDelZip, recortar(&nombre)));
        }
    }
    if vistos.len() != esperados.len() {
        return Err(invalido(MotivoPaquete::EntradasDelZip, "faltan piezas"));
    }
    Ok(())
}

/// Pasos 4 y 5: cada pieza sin pasar de sus `bytes` + 1 ni del total, y con su tamaño y su hash. Las de addon llevan
/// su propio total y además tienen que ser un ZIP de addon seguro; no se guardan.
/// `maximo_total` es el límite del formato; los tests lo bajan para no tener que fabricar 128 MiB.
fn leer_piezas<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    manifiesto: &ManifiestoLeido,
    maximo_total: u64,
) -> Result<HashMap<String, Vec<u8>>, ErrorNucleo> {
    let mut piezas = HashMap::new();
    let mut total: u64 = 0;
    let mut total_addons: u64 = 0;
    for entrada in &manifiesto.entradas {
        let contenido = leer_pieza(zip, entrada)?;
        let nombre = || format!("{CARPETA_DATOS}{}", entrada.id);
        if es_de_addon(entrada) {
            total_addons += entrada.bytes;
            if total_addons > MAX_BYTES_ADDONS {
                return Err(invalido(MotivoPaquete::TotalDemasiadoGrande, nombre()));
            }
            addon_zip::leer(&contenido).map_err(|e| {
                invalido(MotivoPaquete::AddonNoValido, format!("{}: {e}", nombre()))
            })?;
            continue;
        }
        total += entrada.bytes;
        if total > maximo_total {
            return Err(invalido(MotivoPaquete::TotalDemasiadoGrande, nombre()));
        }
        piezas.insert(entrada.id.clone(), contenido);
    }
    Ok(piezas)
}

/// Una pieza sin pasar de sus `bytes` + 1, con su tamaño y su hash comprobados.
fn leer_pieza<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    entrada: &EntradaLeida,
) -> Result<Vec<u8>, ErrorNucleo> {
    let nombre = format!("{CARPETA_DATOS}{}", entrada.id);
    let ilegible = |e: &dyn std::fmt::Display| {
        invalido(MotivoPaquete::PiezaIlegible, format!("{nombre}: {e}"))
    };
    let fichero = zip.by_name(&nombre).map_err(|e| ilegible(&e))?;
    let mut contenido = Vec::new();
    fichero
        .take(entrada.bytes + 1)
        .read_to_end(&mut contenido)
        .map_err(|e| ilegible(&e))?;
    if contenido.len() as u64 > entrada.bytes {
        return Err(invalido(MotivoPaquete::PiezaDemasiadoGrande, nombre));
    }
    if contenido.len() as u64 != entrada.bytes || sha256(&contenido) != entrada.sha256 {
        return Err(invalido(MotivoPaquete::PiezaDanada, nombre));
    }
    Ok(contenido)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paquetes::tests::{entradas_zip, pieza};
    use crate::paquetes::{Origen, Paquete, Perfil};
    use serde_json::{Value, json};
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    fn exportado() -> Vec<u8> {
        let perfil = Perfil {
            nombre: "Druida".into(),
            descripcion: Some("Feral".into()),
            creado: "2026-11-18T10:00:00Z".into(),
        };
        let origen = Origen {
            producto: "wowf",
            build: "1.60.1.70938".into(),
            interfaz: 16001,
        };
        let piezas = vec![pieza("macros", b"VER 3\r\nEND\r\n"), pieza("chat", b"")];
        let zip = Paquete::nuevo(perfil, origen, Vec::new(), piezas)
            .unwrap()
            .a_zip()
            .unwrap();
        // El orden del ZIP no importa al leer: con el manifiesto delante, los tests lo encuentran en `[0]`.
        let mut ficheros = entradas_zip(&zip);
        ficheros.rotate_right(1);
        assert_eq!(ficheros[0].0, MANIFIESTO);
        let ficheros: Vec<_> = ficheros
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        zip_de(&ficheros)
    }

    /// Un ZIP con estos ficheros, en este orden.
    fn zip_de(ficheros: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (nombre, bytes) in ficheros {
            zip.start_file(*nombre, SimpleFileOptions::default())
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    /// El paquete exportado con el manifiesto cambiado por `cambiar`; las piezas, igual.
    fn con_manifiesto(cambiar: impl FnOnce(&mut Value)) -> Vec<u8> {
        let mut ficheros = entradas_zip(&exportado());
        let mut manifiesto: Value = serde_json::from_slice(&ficheros[0].1).unwrap();
        cambiar(&mut manifiesto);
        ficheros[0].1 = serde_json::to_vec(&manifiesto).unwrap();
        let ficheros: Vec<_> = ficheros
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        zip_de(&ficheros)
    }

    fn motivo(bytes: &[u8]) -> MotivoPaquete {
        match leer(bytes) {
            Err(ErrorNucleo::PaqueteInvalido { motivo, .. }) => motivo,
            otro => panic!("se esperaba un paquete inválido: {otro:?}"),
        }
    }

    #[test]
    fn un_paquete_exportado_se_lee_entero() {
        let bytes = exportado();
        let paquete = leer(&bytes).unwrap();
        assert_eq!(paquete.sha256, sha256(&bytes));
        assert_eq!(paquete.manifiesto.perfil.nombre, "Druida");
        assert_eq!(paquete.manifiesto.origen.interfaz, 16001);
        assert_eq!(
            paquete.pieza("macros"),
            Some(b"VER 3\r\nEND\r\n".as_slice())
        );
        assert_eq!(paquete.pieza("chat"), Some(b"".as_slice()));
        assert_eq!(paquete.pieza("otra"), None);
        let macros = &paquete.manifiesto.entradas[0];
        assert_eq!(
            (macros.categoria_conocida(), macros.tipo_conocido()),
            (Some(Categoria::Macros), Some(Tipo::Fichero))
        );
    }

    #[test]
    fn el_ejemplo_del_formato_cumple_el_schema() {
        let documento = include_str!("../../../docs/formato-wowprofile.md");
        let ejemplo = documento.split("## Ejemplo").nth(1).unwrap();
        let json = ejemplo
            .split("```json")
            .nth(1)
            .unwrap()
            .split("```")
            .next()
            .unwrap();
        let manifiesto: ManifiestoLeido = serde_json::from_str(json).unwrap();
        assert_eq!(validar(&manifiesto, 0), Ok(()));
    }

    #[test]
    fn lo_que_no_es_un_zip_se_rechaza() {
        assert_eq!(motivo(b"no soy un zip"), MotivoPaquete::NoEsZip);
    }

    #[test]
    fn mas_de_257_entradas_se_rechaza_antes_de_leer_nada() {
        let nombres: Vec<String> = (0..=MAX_PIEZAS + 1)
            .map(|i| format!("datos/p{i}"))
            .collect();
        let ficheros: Vec<_> = nombres
            .iter()
            .map(|n| (n.as_str(), b"".as_slice()))
            .collect();
        assert_eq!(
            motivo(&zip_de(&ficheros)),
            MotivoPaquete::DemasiadasEntradas
        );
    }

    #[test]
    fn un_manifiesto_que_falta_es_enorme_o_no_es_json_se_rechaza() {
        let enorme = vec![b' '; MAX_BYTES_MANIFIESTO as usize + 1];
        for ficheros in [
            vec![("datos/macros", b"x".as_slice())],
            vec![(MANIFIESTO, enorme.as_slice())],
            vec![(MANIFIESTO, b"{".as_slice())],
        ] {
            assert_eq!(
                motivo(&zip_de(&ficheros)),
                MotivoPaquete::ManifiestoIlegible
            );
        }
    }

    #[test]
    fn otra_version_mayor_pide_actualizar_la_app() {
        let bytes = con_manifiesto(|m| m["formato"]["version"] = json!("2.0"));
        assert_eq!(
            leer(&bytes).unwrap_err(),
            ErrorNucleo::VersionPaquete {
                version: "2.0".into()
            }
        );
        for version in ["01.0", "1", "1.01", "uno.0"] {
            let bytes = con_manifiesto(|m| m["formato"]["version"] = json!(version));
            assert_eq!(
                motivo(&bytes),
                MotivoPaquete::ManifiestoNoValido,
                "{version}"
            );
        }
    }

    #[test]
    fn una_menor_mas_nueva_ignora_lo_desconocido_y_lo_marca() {
        let bytes = con_manifiesto(|m| {
            m["formato"]["version"] = json!("1.2");
            m["novedad"] = json!(true);
            m["entradas"][0]["categoria"] = json!("tablero");
            m["entradas"][0]["tipo"] = json!("parche");
        });
        let paquete = leer(&bytes).unwrap();
        let entrada = &paquete.manifiesto.entradas[0];
        assert_eq!(
            (entrada.categoria_conocida(), entrada.tipo_conocido()),
            (None, None)
        );
    }

    #[test]
    fn en_la_1_0_y_la_1_1_una_categoria_o_un_tipo_desconocidos_no_cumplen_el_schema() {
        for version in ["1.0", "1.1"] {
            for (campo, valor) in [("categoria", "tablero"), ("tipo", "parche")] {
                let bytes = con_manifiesto(|m| {
                    m["formato"]["version"] = json!(version);
                    m["entradas"][0][campo] = json!(valor);
                });
                let error = motivo(&bytes);
                assert_eq!(
                    error,
                    MotivoPaquete::ManifiestoNoValido,
                    "{version} {campo}"
                );
            }
        }
        // La 1.0 no tiene piezas de addon.
        let bytes = con_manifiesto(|m| {
            m["entradas"][0]["categoria"] = json!("addon");
            m["entradas"][0]["tipo"] = json!("carpeta");
        });
        assert_eq!(motivo(&bytes), MotivoPaquete::ManifiestoNoValido);
    }

    /// Un caso con nombre y cómo estropea el manifiesto.
    type Caso = (&'static str, fn(&mut Value));

    #[test]
    fn un_manifiesto_que_no_cumple_el_schema_se_rechaza() {
        let cambios: [Caso; 9] = [
            ("nombre vacío", |m| m["perfil"]["nombre"] = json!("")),
            ("fecha sin Z", |m| {
                m["perfil"]["creado"] = json!("2026-11-18T10:00:00");
            }),
            ("producto", |m| m["origen"]["producto"] = json!("wowc")),
            ("interfaz 0", |m| m["origen"]["interfaz"] = json!(0)),
            ("id repetido", |m| m["entradas"][1]["id"] = json!("macros")),
            ("destino que sube", |m| {
                m["entradas"][0]["destino"] = json!("personaje/../x.txt");
            }),
            ("destino absoluto", |m| {
                m["entradas"][0]["destino"] = json!("C:/WTF/x.txt");
            }),
            ("hash en mayúsculas", |m| {
                let hash = m["entradas"][0]["sha256"].as_str().unwrap().to_uppercase();
                m["entradas"][0]["sha256"] = json!(hash);
            }),
            ("filtrada sin claves", |m| {
                m["entradas"][0]["tipo"] = json!("fichero-filtrado");
            }),
        ];
        for (caso, cambiar) in cambios {
            assert_eq!(
                motivo(&con_manifiesto(cambiar)),
                MotivoPaquete::ManifiestoNoValido,
                "{caso}"
            );
        }
    }

    #[test]
    fn el_zip_tiene_que_traer_exactamente_lo_del_manifiesto() {
        let ficheros = entradas_zip(&exportado());
        let (manifiesto, macros, chat) = (&ficheros[0].1, &ficheros[1].1, &ficheros[2].1);
        let casos: [Vec<(&str, &[u8])>; 3] = [
            vec![(MANIFIESTO, manifiesto), ("datos/macros", macros)],
            vec![
                (MANIFIESTO, manifiesto),
                ("datos/macros", macros),
                ("datos/chat", chat),
                ("../fuera", b"x"),
            ],
            vec![
                (MANIFIESTO, manifiesto),
                ("datos/macros", macros),
                ("datos/chat", chat),
                ("datos/", b""),
            ],
        ];
        for ficheros in casos {
            assert_eq!(motivo(&zip_de(&ficheros)), MotivoPaquete::EntradasDelZip);
        }
    }

    #[test]
    fn una_pieza_que_descomprime_de_mas_se_corta_y_se_rechaza() {
        let bytes = con_manifiesto(|m| m["entradas"][0]["bytes"] = json!(3));
        assert_eq!(motivo(&bytes), MotivoPaquete::PiezaDemasiadoGrande);
    }

    #[test]
    fn una_pieza_con_otro_hash_o_mas_corta_se_rechaza() {
        let otro_hash = con_manifiesto(|m| m["entradas"][1]["sha256"] = json!(sha256(b"x")));
        let mas_larga_declarada = con_manifiesto(|m| m["entradas"][1]["bytes"] = json!(5));
        for bytes in [otro_hash, mas_larga_declarada] {
            assert_eq!(motivo(&bytes), MotivoPaquete::PiezaDanada);
        }
    }

    #[test]
    fn un_fichero_mayor_que_el_maximo_no_se_lee() {
        let tmp = tempfile::tempdir().unwrap();
        let ruta = tmp.path().join("enorme.wowprofile");
        let fichero = std::fs::File::create(&ruta).unwrap();
        fichero.set_len(MAX_BYTES_FICHERO + 1).unwrap();
        match leer_fichero(&ruta) {
            Err(ErrorNucleo::PaqueteInvalido { motivo, .. }) => {
                assert_eq!(motivo, MotivoPaquete::DemasiadoGrande);
            }
            otro => panic!("{otro:?}"),
        }
    }

    #[test]
    fn en_la_1_0_un_null_un_campo_de_mas_o_un_marcador_raro_no_cumplen_el_schema() {
        let casos: [Caso; 5] = [
            ("null opcional", |m| {
                m["perfil"]["descripcion"] = Value::Null
            }),
            ("campo en la raíz", |m| m["firma"] = json!("x")),
            ("campo en una entrada", |m| {
                m["entradas"][0]["ruta"] = json!("C:/x")
            }),
            ("marcador desconocido", |m| {
                m["entradas"][0]["marcadores"] = json!(["CUENTA"]);
            }),
            ("marcador repetido", |m| {
                m["entradas"][0]["marcadores"] = json!(["REINO", "REINO"]);
            }),
        ];
        for (caso, cambiar) in casos {
            assert_eq!(
                motivo(&con_manifiesto(cambiar)),
                MotivoPaquete::ManifiestoNoValido,
                "{caso}"
            );
        }
        let con_marcador =
            con_manifiesto(|m| m["entradas"][0]["marcadores"] = json!(["PERSONAJE"]));
        assert!(leer(&con_marcador).is_ok());
    }

    #[test]
    fn un_nombre_de_zip_enorme_no_viaja_entero_en_el_error() {
        let largo = format!("datos/{}", "x".repeat(10_000));
        let ficheros = entradas_zip(&exportado());
        let zip = zip_de(&[
            (MANIFIESTO, ficheros[0].1.as_slice()),
            ("datos/macros", ficheros[1].1.as_slice()),
            ("datos/chat", ficheros[2].1.as_slice()),
            (largo.as_str(), b""),
        ]);
        match leer(&zip) {
            Err(ErrorNucleo::PaqueteInvalido { detalle, .. }) => {
                assert_eq!(detalle.chars().count(), MAX_DIAGNOSTICO);
            }
            otro => panic!("{otro:?}"),
        }
    }

    #[test]
    fn las_piezas_no_pasan_del_total() {
        let bytes = exportado();
        let total = "VER 3\r\nEND\r\n".len() as u64;
        assert!(leer_con_tope(&bytes, total).is_ok());
        match leer_con_tope(&bytes, total - 1) {
            Err(ErrorNucleo::PaqueteInvalido { motivo, .. }) => {
                assert_eq!(motivo, MotivoPaquete::TotalDemasiadoGrande);
            }
            otro => panic!("{otro:?}"),
        }
    }

    #[test]
    fn destinos_y_fechas_como_en_el_schema() {
        for valido in [
            "personaje/macros-cache.txt",
            "cuenta/SavedVariables/Prat-3.0.lua",
            "personaje/CUFProfiles.txt",
        ] {
            assert!(destino_valido(valido), "{valido}");
        }
        for no_valido in [
            "personaje/",
            "personaje/sin-extension",
            "personaje/a.exe",
            "personaje/sub/a.txt",
            "personaje/.txt",
            "personaje/a..txt",
            "otro/a.txt",
            "personaje\\a.txt",
        ] {
            assert!(!destino_valido(no_valido), "{no_valido}");
        }
        assert!(fecha_utc("2026-11-18T10:00:00Z"));
        assert!(fecha_utc("2026-11-18T10:00:00.123Z"));
        assert!(fecha_utc("2028-02-29T10:00:00Z"));
        for no_valida in [
            "2026-02-29T10:00:00Z",
            "2026-04-31T10:00:00Z",
            "2026-11-18T10:00:00",
            "2026-13-18T10:00:00Z",
            "2026-11-18 10:00:00Z",
            "2026-11-18T10:00:00.Z",
        ] {
            assert!(!fecha_utc(no_valida), "{no_valida}");
        }
    }
}
