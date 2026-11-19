//! Modo Edición de punta a punta (spec 034): el diseño del origen se lleva en un `.wowprofile` real, se aplica en dos
//! personajes de la misma cuenta y deshacer devuelve los ficheros como estaban.

use super::super::{PaquetesAbiertos, Pedido};
use super::*;
use crate::deteccion::{PersonajeUbicado, Producto};
use crate::modo_edicion;
use crate::operaciones::Operaciones;
use crate::paquetes::{
    Ambito, Categoria, DeAddon, Origen, Paquete, Perfil, Pieza, RefAdaptador, Tipo,
};
use std::fs::{create_dir_all, read, write};
use std::path::{Path, PathBuf};

const CUENTA: &str = "1#1";
/// La cuenta del origen: `default` y `Sanador`, el que usa el personaje que exporta.
const ORIGEN: &str = "5 1 0 7 default 0 1 0 0 1 7 7 UIParent 0.0 -4.0 -1 ## 7 Sanador 0 1 0 0 1 7 7 UIParent 1.0 2.0 -1 #$\0";
/// La cuenta del destino: su propio `default`, distinto.
const DESTINO: &str = "5 1 0 7 default 0 1 0 0 1 7 7 UIParent 9.0 9.0 -1 ##\0";
const USA_DEFAULT: &[u8] = b"5 3 3 3 3 3 -1 -1 -1 -1 -1\0";

fn paquete(fichero: &Path) {
    let (nombre, contenido) =
        modo_edicion::exportar(ORIGEN.as_bytes(), b"5 4 3 3 3 3 -1 -1 -1 -1 -1\0")
            .unwrap()
            .unwrap();
    let pieza = Pieza {
        id: "modo-edicion".into(),
        categoria: Categoria::ModoEdicion,
        tipo: Tipo::Adaptador,
        ambito: Ambito::Cuenta,
        destino: "cuenta/edit-mode-cache-account.txt".into(),
        contenido,
        quitadas: Vec::new(),
        addon: Some(DeAddon {
            perfil: Some(nombre),
            adaptador: Some(RefAdaptador {
                id: modo_edicion::ADAPTADOR.into(),
                version: modo_edicion::VERSION_ADAPTADOR,
            }),
            ..DeAddon::default()
        }),
    };
    let perfil = Perfil {
        nombre: "Sanador".into(),
        descripcion: None,
        creado: "2026-11-19T10:00:00Z".into(),
    };
    let origen = Origen {
        producto: "wowf",
        build: "1.60.1.70959".into(),
        interfaz: 16001,
    };
    Paquete::nuevo(perfil, origen, Vec::new(), vec![pieza])
        .unwrap()
        .guardar(fichero)
        .unwrap();
}

fn pj(raiz: &Path, carpeta: &str) -> PersonajeUbicado {
    PersonajeUbicado {
        raiz_version: raiz.to_path_buf(),
        producto: Producto::Forever,
        cuenta: CUENTA.into(),
        reino: "93".into(),
        personaje: carpeta.into(),
        nombre_en_datos: None,
    }
}

fn del_personaje(raiz: &Path, carpeta: &str) -> Vec<u8> {
    read(
        pj(raiz, carpeta)
            .carpeta()
            .join(modo_edicion::FICHERO_PERSONAJE),
    )
    .unwrap()
}

#[test]
fn se_lleva_el_diseno_se_aplica_en_dos_personajes_y_se_deshace() {
    let tmp = tempfile::tempdir().unwrap();
    let raiz: PathBuf = tmp.path().join("_forever_");
    create_dir_all(raiz.join("Interface/AddOns")).unwrap();
    write(
        raiz.join(".flavor.info"),
        "Product Flavor!STRING:0\r\nwowf\r\n",
    )
    .unwrap();
    for carpeta in ["A", "B"] {
        let carpeta = pj(&raiz, carpeta).carpeta();
        create_dir_all(&carpeta).unwrap();
        write(carpeta.join(modo_edicion::FICHERO_PERSONAJE), USA_DEFAULT).unwrap();
    }
    let de_cuenta = pj(&raiz, "A")
        .carpeta_cuenta()
        .join("edit-mode-cache-account.txt");
    write(&de_cuenta, DESTINO).unwrap();
    let fichero = tmp.path().join("sanador.wowprofile");
    paquete(&fichero);
    let abiertos = PaquetesAbiertos::default();
    let sha256 = abiertos.abrir(&fichero).unwrap().sha256;
    let operaciones = Operaciones::en(tmp.path().join("operaciones"));
    let pedido = Pedido {
        seleccion: vec!["modo-edicion".into()],
        cuentas: Vec::new(),
        perfiles: Vec::new(),
    };
    let personajes = || vec![pj(&raiz, "A"), pj(&raiz, "B")];

    let huella = abiertos
        .vista_previa(&sha256, personajes(), &pedido, false)
        .unwrap()
        .huella;
    let resultados = abiertos
        .aplicar(
            &sha256,
            &huella,
            personajes(),
            &pedido,
            &operaciones,
            || false,
        )
        .unwrap();

    let escritos: Vec<_> = resultados
        .iter()
        .map(|r| match &r.resultado {
            Resultado::Completa { ficheros, .. } => *ficheros,
            otro => panic!("{otro:?}"),
        })
        .collect();
    assert_eq!(escritos, [2, 1], "la cuenta la escribe solo el primero");
    let cuenta = String::from_utf8(read(&de_cuenta).unwrap()).unwrap();
    let disenos = modo_edicion::Cuenta::leer(&cuenta).unwrap();
    let nombres: Vec<_> = disenos.disenos.iter().map(|d| d.nombre.as_str()).collect();
    assert_eq!(nombres, ["default", "Sanador"]);
    assert!(cuenta.starts_with(DESTINO.trim_end_matches('\0')));
    for carpeta in ["A", "B"] {
        assert_eq!(
            del_personaje(&raiz, carpeta),
            b"5 4 3 3 3 3 -1 -1 -1 -1 -1\0",
            "{carpeta}"
        );
    }

    // Deshacer el segundo y luego el primero deja todo como estaba.
    for resultado in resultados.iter().rev() {
        let Resultado::Completa { operacion, .. } = &resultado.resultado else {
            unreachable!()
        };
        let plan = operaciones.plan_deshacer(operacion).unwrap();
        operaciones
            .deshacer(operacion, &plan.huella, || false)
            .unwrap();
    }
    assert_eq!(read(&de_cuenta).unwrap(), DESTINO.as_bytes());
    assert_eq!(del_personaje(&raiz, "A"), USA_DEFAULT);
    assert_eq!(del_personaje(&raiz, "B"), USA_DEFAULT);
}
