//! Fusionar al importar (#72, decisión 3): un valor del paquete dentro del mismo valor del destino, con las rutas de
//! una regla. En el destino solo cambia lo que trae el paquete y deja pasar la regla; lo demás queda byte a byte y nada
//! se borra. Solo se escriben de nuevo las llaves de las tablas en las que algo cambia.

use super::{
    CLAVE_NAMESPACES, CLAVE_PERFIL, CLAVE_PERFILES, CLAVE_PERSONAJES, Documento,
    MARCADOR_PERSONAJE, Recorte, Rutas, Seleccion, clave_entre_corchetes, clave_y_valor,
};
use full_moon::ast::{Expression, Field, TableConstructor};
use full_moon::node::Node;

/// El fichero del destino con la variable `nombre` del paquete fusionada según `seleccion`, cuya `personaje` es la clave
/// del personaje destino (Latin-1, tal como va entre comillas). En el paquete, esa clave es el marcador. `None` si el
/// paquete no trae esa variable o nada de ella que la regla deje pasar.
pub fn fusionar_variable(
    destino: &Documento,
    paquete: &Documento,
    nombre: &str,
    seleccion: &Seleccion<'_>,
) -> Option<Vec<u8>> {
    let valor = paquete.variable(nombre)?.valor;
    fusionar_en(
        destino,
        nombre,
        paquete,
        valor,
        &Rutas::de(seleccion),
        seleccion.personaje,
    )
}

/// Un perfil AceDB que se importa: la pieza (`Documento::leer_valor`), con qué nombre queda y para qué personaje. Los
/// textos van en Latin-1 y tal como se escriben entre comillas.
pub struct PerfilAcedb<'a> {
    pub variable: &'a str,
    pub pieza: &'a Documento,
    pub nombre: &'a str,
    pub personaje: &'a str,
}

/// El fichero del destino con el perfil en `profiles`, el de cada módulo en `namespaces.<módulo>.profiles` y
/// `profileKeys` del personaje apuntando a él. Las demás entradas, también las de otros personajes, no cambian. `None`
/// si la pieza no tiene la forma de `docs/formato-wowprofile.md`.
pub fn fusionar_perfil(destino: &Documento, perfil: &PerfilAcedb<'_>) -> Option<Vec<u8>> {
    let sintetico = Documento::leer_valor(&tabla_de_perfil(perfil)?)?;
    let valor = sintetico.valor_suelto()?.valor;
    let mut rutas = vec![
        vec![CLAVE_PERSONAJES, perfil.personaje],
        vec![CLAVE_PERFILES, perfil.nombre],
    ];
    let modulos = modulos_de(perfil.pieza);
    for modulo in &modulos {
        rutas.push(vec![
            CLAVE_NAMESPACES,
            modulo.as_str(),
            CLAVE_PERFILES,
            perfil.nombre,
        ]);
    }
    let rutas = Rutas {
        incluir: Some(rutas),
        excluir: Vec::new(),
        solo: Vec::new(),
    };
    fusionar_en(
        destino,
        perfil.variable,
        &sintetico,
        valor,
        &rutas,
        perfil.personaje,
    )
}

/// Los módulos de una pieza `perfil-acedb`, con su clave en Latin-1.
fn modulos_de(pieza: &Documento) -> Vec<String> {
    pieza
        .valor_suelto()
        .and_then(|v| super::tabla(v.valor))
        .and_then(|t| super::campo(t, CLAVE_NAMESPACES))
        .and_then(super::tabla)
        .into_iter()
        .flat_map(super::campos)
        .map(|(modulo, _)| modulo)
        .collect()
}

/// `{ profileKeys = { [pj] = "perfil" }, profiles = { [perfil] = … }, namespaces = { [m] = { profiles = { [perfil] = … } } } }`
/// con los bytes de la pieza: la forma del destino, para fusionarla con las rutas de cada parte.
fn tabla_de_perfil(perfil: &PerfilAcedb<'_>) -> Option<Vec<u8>> {
    let raiz = super::tabla(perfil.pieza.valor_suelto()?.valor)?;
    let fin = perfil.pieza.fin_de_linea().as_bytes();
    let entrada = |salida: &mut Vec<u8>, clave: &str, valor: &[u8]| {
        salida.extend(clave_entre_corchetes(clave));
        salida.extend(b" = ");
        salida.extend(valor);
        salida.extend(b",");
        salida.extend(fin);
    };
    let abrir = |salida: &mut Vec<u8>, clave: &str| {
        salida.extend(clave_entre_corchetes(clave));
        salida.extend(b" = {");
        salida.extend(fin);
    };
    let cerrar = |salida: &mut Vec<u8>| {
        salida.extend(b"},");
        salida.extend(fin);
    };
    let mut nombre = b"\"".to_vec();
    nombre.extend(super::a_bytes(perfil.nombre));
    nombre.extend(b"\"");
    let mut salida = b"{".to_vec();
    salida.extend(fin);
    abrir(&mut salida, CLAVE_PERSONAJES);
    entrada(&mut salida, perfil.personaje, &nombre);
    cerrar(&mut salida);
    abrir(&mut salida, CLAVE_PERFILES);
    let perfil_bytes = perfil
        .pieza
        .bytes_de(super::campo(raiz, CLAVE_PERFIL).filter(|v| super::tabla(v).is_some())?);
    entrada(&mut salida, perfil.nombre, &perfil_bytes);
    cerrar(&mut salida);
    let modulos: Vec<_> = super::campo(raiz, CLAVE_NAMESPACES)
        .and_then(super::tabla)
        .into_iter()
        .flat_map(super::campos)
        .collect();
    if !modulos.is_empty() {
        abrir(&mut salida, CLAVE_NAMESPACES);
        for (modulo, valor) in modulos {
            super::tabla(valor)?;
            abrir(&mut salida, &modulo);
            abrir(&mut salida, CLAVE_PERFILES);
            entrada(&mut salida, perfil.nombre, &perfil.pieza.bytes_de(valor));
            cerrar(&mut salida);
            cerrar(&mut salida);
        }
        cerrar(&mut salida);
    }
    salida.extend(b"}");
    Some(salida)
}

/// La variable `nombre` del destino con `valor` (de `paquete`) fusionado según `rutas`. `personaje`: la clave del
/// personaje destino.
fn fusionar_en(
    destino: &Documento,
    nombre: &str,
    paquete: &Documento,
    valor: &Expression,
    rutas: &Rutas<'_>,
    personaje: &str,
) -> Option<Vec<u8>> {
    let fusion = Fusion {
        destino,
        paquete,
        personaje,
    };
    let actual = destino.variable(nombre).map(|v| v.valor);
    let nuevo = fusion.valor(actual, valor, rutas)?;
    Some(destino.con_variable(nombre, &nuevo))
}

struct Fusion<'d> {
    destino: &'d Documento,
    paquete: &'d Documento,
    /// La clave del personaje destino.
    personaje: &'d str,
}

impl Fusion<'_> {
    /// Recorte del destino: reconoce la clave del personaje destino.
    fn del_destino(&self) -> Recorte<'_> {
        Recorte {
            documento: self.destino,
            personaje: self.personaje,
            salida_personaje: self.personaje,
        }
    }

    /// Recorte del paquete: su clave de personaje es el marcador, y se escribe con la del destino.
    fn del_paquete(&self) -> Recorte<'_> {
        Recorte {
            documento: self.paquete,
            personaje: MARCADOR_PERSONAJE,
            salida_personaje: self.personaje,
        }
    }

    /// El valor fusionado. Con `destino` siempre hay valor; sin él, lo que el paquete trae y la regla deja pasar.
    fn valor(
        &self,
        destino: Option<&Expression>,
        paquete: &Expression,
        rutas: &Rutas<'_>,
    ) -> Option<Vec<u8>> {
        match (destino, paquete) {
            (Some(Expression::TableConstructor(d)), Expression::TableConstructor(p))
                if !rutas.entero() =>
            {
                Some(self.tabla(d, p, rutas))
            }
            (Some(d), p) => self
                .del_paquete()
                .valor(p, rutas)
                .or_else(|| Some(self.destino.bytes_de(d))),
            (None, p) => self.del_paquete().valor(p, rutas),
        }
    }

    /// Una tabla del destino con la del paquete dentro. Si nada cambia, sus bytes tal cual. Si cambia, se edita en su
    /// sitio: cada valor que cambia se sustituye y lo nuevo se añade antes de la llave de cierre, con la sangría de la
    /// tabla. Las entradas que no cambian conservan sus bytes, espacios y comentarios incluidos.
    fn tabla(
        &self,
        destino: &TableConstructor,
        paquete: &TableConstructor,
        rutas: &Rutas<'_>,
    ) -> Vec<u8> {
        let (listas, listas_cambian) = self.listas(destino, paquete, rutas);
        let del_destino = self.del_destino();
        let mut del_paquete: Vec<_> = paquete
            .fields()
            .iter()
            .filter_map(|campo| {
                let (clave, valor) = clave_y_valor(campo)?;
                let clave = if clave == MARCADOR_PERSONAJE {
                    self.personaje.to_owned()
                } else {
                    clave
                };
                Some((clave, campo, valor, false))
            })
            .collect();
        let mut ediciones = Vec::new();
        for campo in destino.fields() {
            let Some((clave, actual)) = clave_y_valor(campo) else {
                continue;
            };
            let viaja = del_destino.hijas(rutas, &clave);
            let par = del_paquete.iter_mut().find(|(k, ..)| *k == clave);
            let (Some((hijas, _)), Some((_, _, valor, usado))) = (viaja, par) else {
                continue;
            };
            *usado = true;
            let anterior = self.destino.bytes_de(actual);
            let Some(nuevo) = self.valor(Some(actual), valor, &hijas) else {
                continue;
            };
            if let (true, Some((inicio, fin))) = (nuevo != anterior, actual.range()) {
                ediciones.push((inicio.bytes(), fin.bytes(), nuevo));
            }
        }
        let nuevos: Vec<Vec<u8>> = del_paquete
            .iter()
            .filter(|(.., usado)| !usado)
            .filter_map(|(clave, campo, valor, _)| self.nuevo_campo(clave, campo, valor, rutas))
            .collect();
        if listas_cambian {
            return self.tabla_rehecha(destino, listas, &ediciones, &nuevos);
        }
        if !nuevos.is_empty() {
            ediciones.push(self.insercion(destino, &nuevos));
        }
        if ediciones.is_empty() {
            return self.destino.bytes_de(destino);
        }
        self.destino.editado(destino, ediciones)
    }

    /// Dónde y qué se inserta para añadir `nuevos` al final de una tabla del destino.
    fn insercion(&self, destino: &TableConstructor, nuevos: &[Vec<u8>]) -> (usize, usize, Vec<u8>) {
        let fin = self.destino.fin_de_linea().as_bytes();
        let ultimo = destino.fields().pairs().last();
        let (posicion, sangria, falta_coma) = match ultimo {
            Some(par) => {
                let campo = par.value();
                let inicio = campo.range().map_or(0, |(i, _)| i.bytes());
                let (posicion, falta) = match par.punctuation() {
                    Some(coma) => (coma.range().map_or(0, |(_, f)| f.bytes()), false),
                    None => (campo.range().map_or(0, |(_, f)| f.bytes()), true),
                };
                (posicion, self.destino.sangria(inicio), falta)
            }
            None => {
                let (abre, cierra) = destino.braces().tokens();
                let posicion = abre.range().map_or(0, |(_, f)| f.bytes());
                let inicio_cierre = cierra.range().map_or(0, |(i, _)| i.bytes());
                (
                    posicion,
                    format!("{}\t", self.destino.sangria(inicio_cierre)),
                    false,
                )
            }
        };
        let mut texto = Vec::new();
        if falta_coma {
            texto.extend(b",");
        }
        for nuevo in nuevos {
            texto.extend(fin);
            texto.extend(super::a_bytes(&sangria));
            texto.extend(nuevo);
            texto.extend(b",");
        }
        (posicion, posicion, texto)
    }

    /// La tabla escrita de nuevo, para cuando cambian sus entradas sin clave (que van por posición).
    fn tabla_rehecha(
        &self,
        destino: &TableConstructor,
        listas: Vec<Vec<u8>>,
        ediciones: &[(usize, usize, Vec<u8>)],
        nuevos: &[Vec<u8>],
    ) -> Vec<u8> {
        let fin = self.destino.fin_de_linea().as_bytes();
        let mut cuerpo = Vec::new();
        let mut linea = |bytes: Vec<u8>| {
            cuerpo.extend(bytes);
            cuerpo.extend(b",");
            cuerpo.extend(fin);
        };
        listas.into_iter().for_each(&mut linea);
        for campo in destino.fields() {
            if matches!(campo, Field::NoKey(_)) {
                continue;
            }
            let rango = campo.range().map(|(i, f)| (i.bytes(), f.bytes()));
            let suyas: Vec<_> = ediciones
                .iter()
                .filter(|(i, f, _)| rango.is_some_and(|(ci, cf)| ci <= *i && *f <= cf))
                .cloned()
                .collect();
            linea(self.destino.editado(campo, suyas));
        }
        nuevos.iter().cloned().for_each(&mut linea);
        let mut salida = b"{".to_vec();
        salida.extend(fin);
        salida.extend(cuerpo);
        salida.extend(b"}");
        salida
    }

    /// Las entradas sin clave que quedan: las del paquete si este nivel viaja entero y trae alguna, si no las del
    /// destino. Un paquete sin lista no borra la del destino: con una regla `excluir`, una tabla que solo se ve a medias
    /// no basta para decir que su lista está vacía. Por eso un `{}` del paquete tampoco vacía una lista: nada se borra.
    /// El `bool` dice si cambian.
    fn listas(
        &self,
        destino: &TableConstructor,
        paquete: &TableConstructor,
        rutas: &Rutas<'_>,
    ) -> (Vec<Vec<u8>>, bool) {
        let sin_clave = |documento: &Documento, tabla: &TableConstructor| -> Vec<Vec<u8>> {
            tabla
                .fields()
                .iter()
                .filter(|c| matches!(c, Field::NoKey(_)))
                .map(|c| documento.bytes_de(c))
                .collect()
        };
        let actuales = sin_clave(self.destino, destino);
        let todo = rutas
            .incluir
            .as_ref()
            .is_none_or(|r| r.iter().any(Vec::is_empty));
        if !todo {
            return (actuales, false);
        }
        let nuevas = sin_clave(self.paquete, paquete);
        if nuevas.is_empty() {
            return (actuales, false);
        }
        let cambia = nuevas != actuales;
        (nuevas, cambia)
    }

    /// `clave = valor` de un campo del paquete que el destino no tiene, si la regla lo deja pasar.
    fn nuevo_campo(
        &self,
        clave: &str,
        campo: &Field,
        valor: &Expression,
        rutas: &Rutas<'_>,
    ) -> Option<Vec<u8>> {
        let (hijas, es_personaje) = self.del_destino().hijas(rutas, clave)?;
        let texto = self.del_paquete().valor(valor, &hijas)?;
        let mut salida = if es_personaje || clave == self.personaje {
            clave_entre_corchetes(self.personaje)
        } else {
            self.paquete.bytes_de_clave(campo)
        };
        salida.extend(b" = ");
        salida.extend(texto);
        Some(salida)
    }
}

#[cfg(test)]
mod tests;
