//! La configuración de addons de punta a punta (#72): plan, aplicar y deshacer sobre una instalación en una carpeta
//! temporal, con dos destinos de una misma cuenta y un tercer personaje que no se toca.

use super::super::{AccionPerfil, AvisoFichero, ResultadoPersonaje};
use super::super::{DecisionCuenta, DecisionPerfil, Grupo, PaquetesAbiertos, Pedido, VistaPrevia};
use crate::deteccion::{PersonajeUbicado, Producto};
use crate::importacion::aplicar::Resultado;
use crate::lua::{Documento, MARCADOR_PERSONAJE};
use crate::operaciones::Operaciones;
use crate::paquetes::{Ambito, Categoria, DeAddon, Marcador, Origen, Paquete, Perfil, Pieza, Tipo};
use std::collections::BTreeMap;
use std::fs::{create_dir_all, read, write};
use std::path::{Path, PathBuf};

const CUENTA: &str = "1#1";
const CUENTA_LUA: &str = concat!(
    "\r\nDominosDB = {\r\n\t[\"profileKeys\"] = {\r\n\t\t[\"Tercero Pj\"] = \"Default\",\r\n\t},\r\n",
    "\t[\"profiles\"] = {\r\n\t\t[\"Default\"] = {\r\n\t\t\t[\"barras\"] = 1,\r\n\t\t},\r\n\t},\r\n",
    "\t[\"char\"] = {\r\n\t\t[\"Tercero Pj\"] = {\r\n\t\t\t[\"oro\"] = 7,\r\n\t\t},\r\n\t},\r\n}\r\n",
    "DominosVersion = 1\r\n"
);
const PERFIL: &[u8] = b"{\r\n[\"profile\"] = {\r\n[\"barras\"] = 5,\r\n},\r\n}\r\n";
const GENERICO_CUENTA: &[u8] = b"DominosVersion = 2\r\n";
const GENERICO_PJ: &[u8] = b"DominosCharDB = {\r\n[\"x\"] = 1,\r\n}\r\n";

struct Mundo {
    _tmp: tempfile::TempDir,
    raiz: PathBuf,
    abiertos: PaquetesAbiertos,
    operaciones: Operaciones,
    sha256: String,
}

fn pieza(id: &str, tipo: Tipo, ambito: Ambito, contenido: &[u8], de_addon: DeAddon) -> Pieza {
    let carpeta = match ambito {
        Ambito::Cuenta => "cuenta",
        Ambito::Personaje => "personaje",
    };
    Pieza {
        id: id.to_owned(),
        categoria: Categoria::ConfigAddon,
        tipo,
        ambito,
        destino: format!("{carpeta}/SavedVariables/Dominos.lua"),
        contenido: contenido.to_vec(),
        quitadas: Vec::new(),
        addon: Some(de_addon),
    }
}

fn dominos() -> DeAddon {
    DeAddon {
        addon: "Dominos".into(),
        ..DeAddon::default()
    }
}

fn guardar_paquete(fichero: &Path) {
    let perfil = Perfil {
        nombre: "Barras".into(),
        descripcion: None,
        creado: "2026-11-18T10:00:00Z".into(),
    };
    let origen = Origen {
        producto: "wowf",
        build: "1.60.1.70938".into(),
        interfaz: 16001,
    };
    let piezas = vec![
        pieza(
            "addon-dominos-perfil",
            Tipo::PerfilAcedb,
            Ambito::Cuenta,
            PERFIL,
            DeAddon {
                variable: Some("DominosDB".into()),
                perfil: Some(MARCADOR_PERSONAJE.into()),
                marcadores: vec![Marcador::Personaje],
                ..dominos()
            },
        ),
        pieza(
            "addon-dominos-cuenta",
            Tipo::Fichero,
            Ambito::Cuenta,
            GENERICO_CUENTA,
            dominos(),
        ),
        pieza(
            "addon-dominos-personaje",
            Tipo::Fichero,
            Ambito::Personaje,
            GENERICO_PJ,
            dominos(),
        ),
    ];
    Paquete::nuevo(perfil, origen, Vec::new(), piezas)
        .unwrap()
        .guardar(fichero)
        .unwrap();
}

/// Forever con Dominos sin instalar y tres personajes en `1#1`. El fichero de cuenta de Dominos ya tiene al tercero.
fn mundo() -> Mundo {
    let tmp = tempfile::tempdir().unwrap();
    let raiz = tmp.path().join("_forever_");
    create_dir_all(raiz.join("Interface/AddOns")).unwrap();
    write(
        raiz.join(".flavor.info"),
        "Product Flavor!STRING:0\r\nwowf\r\n",
    )
    .unwrap();
    let cuenta = raiz.join("WTF/Account").join(CUENTA);
    for carpeta in ["A", "B", "C"] {
        create_dir_all(cuenta.join("93").join(carpeta)).unwrap();
    }
    create_dir_all(cuenta.join("SavedVariables")).unwrap();
    write(cuenta.join("SavedVariables/Dominos.lua"), CUENTA_LUA).unwrap();
    let fichero = tmp.path().join("barras.wowprofile");
    guardar_paquete(&fichero);
    let abiertos = PaquetesAbiertos::default();
    let sha256 = abiertos.abrir(&fichero).unwrap().sha256;
    Mundo {
        operaciones: Operaciones::en(tmp.path().join("operaciones")),
        _tmp: tmp,
        raiz,
        abiertos,
        sha256,
    }
}

fn pj(m: &Mundo, carpeta: &str, nombre: &str) -> PersonajeUbicado {
    PersonajeUbicado {
        raiz_version: m.raiz.clone(),
        producto: Producto::Forever,
        cuenta: CUENTA.into(),
        reino: "93".into(),
        personaje: carpeta.into(),
        nombre_en_datos: Some(nombre.into()),
    }
}

fn destinos(m: &Mundo) -> Vec<PersonajeUbicado> {
    vec![pj(m, "A", "Goldberg Joestar"), pj(m, "B", "Otra Pj")]
}

fn pedido(aceptar_cuenta: bool) -> Pedido {
    Pedido {
        seleccion: vec![
            "addon-dominos-perfil".into(),
            "addon-dominos-cuenta".into(),
            "addon-dominos-personaje".into(),
        ],
        cuentas: vec![DecisionCuenta {
            cuenta: CUENTA.into(),
            entrada: "addon-dominos-cuenta".into(),
            aceptar: aceptar_cuenta,
        }],
        perfiles: Vec::new(),
    }
}

fn vista(m: &Mundo, pedido: &Pedido) -> VistaPrevia {
    m.abiertos
        .vista_previa(&m.sha256, destinos(m), pedido, false)
        .unwrap()
}

/// Todos los ficheros del `WTF`, con su contenido.
fn instantanea(m: &Mundo) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut ficheros = BTreeMap::new();
    let mut pendientes = vec![m.raiz.join("WTF")];
    while let Some(carpeta) = pendientes.pop() {
        for entrada in std::fs::read_dir(&carpeta).unwrap().flatten() {
            let ruta = entrada.path();
            if ruta.is_dir() {
                pendientes.push(ruta);
            } else {
                ficheros.insert(ruta.clone(), read(&ruta).unwrap());
            }
        }
    }
    ficheros
}

fn cuenta_lua(m: &Mundo) -> String {
    String::from_utf8(read(m.raiz.join("WTF/Account/1#1/SavedVariables/Dominos.lua")).unwrap())
        .unwrap()
}

#[test]
fn el_fichero_de_cuenta_se_planifica_una_vez_con_todos_los_destinos() {
    let m = mundo();
    let plan = vista(&m, &pedido(true)).plan;
    let a = &plan.personajes[0];
    let de_cuenta: Vec<_> = a.ficheros.iter().filter(|f| f.en_cuenta).collect();
    assert_eq!(de_cuenta.len(), 1, "un fichero, una escritura");
    let fichero = de_cuenta[0];
    assert_eq!(
        fichero.entradas,
        ["addon-dominos-perfil", "addon-dominos-cuenta"]
    );
    assert_eq!(fichero.grupo, Grupo::Modificado);
    assert!(fichero.de_cuenta, "lleva un genérico de cuenta");
    assert_eq!(
        fichero.avisos,
        [AvisoFichero::FaltaAddon {
            addon: "Dominos".into()
        }]
    );
    let b = &plan.personajes[1];
    let en_b = b.ficheros.iter().find(|f| f.en_cuenta).unwrap();
    assert_eq!(
        (en_b.grupo, en_b.escrita_en.as_deref()),
        (Grupo::SinCambios, Some("A"))
    );
    // Cada destino tiene además su fichero de personaje.
    assert!(
        a.ficheros
            .iter()
            .any(|f| !f.en_cuenta && f.grupo == Grupo::Nuevo)
    );
    assert!(
        b.ficheros
            .iter()
            .any(|f| !f.en_cuenta && f.grupo == Grupo::Nuevo)
    );
    assert!(a.conflictos.is_empty() && a.conflictos_perfil.is_empty());
}

#[test]
fn sin_decidir_la_cuenta_el_generico_es_conflicto_y_el_perfil_sigue() {
    let m = mundo();
    let mut sin_decidir = pedido(true);
    sin_decidir.cuentas.clear();
    let plan = vista(&m, &sin_decidir).plan;
    for personaje in &plan.personajes {
        assert_eq!(personaje.conflictos, ["addon-dominos-cuenta"]);
    }
    let fichero = plan.personajes[0]
        .ficheros
        .iter()
        .find(|f| f.en_cuenta)
        .unwrap();
    assert_eq!(fichero.entradas, ["addon-dominos-perfil"]);
}

#[test]
fn aplicar_toca_solo_a_los_destinos_y_deshacer_lo_deja_byte_a_byte() {
    let m = mundo();
    let antes = instantanea(&m);
    let pedido = pedido(true);
    let huella = vista(&m, &pedido).huella;

    let resultados: Vec<ResultadoPersonaje> = m
        .abiertos
        .aplicar(
            &m.sha256,
            &huella,
            destinos(&m),
            &pedido,
            &m.operaciones,
            || false,
        )
        .unwrap();

    let ids: Vec<String> = resultados
        .iter()
        .map(|r| match &r.resultado {
            Resultado::Completa { operacion, .. } => operacion.clone(),
            otro => panic!("{otro:?}"),
        })
        .collect();
    let lua = cuenta_lua(&m);
    // El tercero, su perfil y su estado, tal cual; los dos destinos con un perfil con su nombre.
    assert!(lua.contains("\t\t[\"Tercero Pj\"] = \"Default\",\r\n"));
    assert!(lua.contains("\t\t[\"Default\"] = {\r\n\t\t\t[\"barras\"] = 1,\r\n\t\t},\r\n"));
    assert!(lua.contains(
        "\t[\"char\"] = {\r\n\t\t[\"Tercero Pj\"] = {\r\n\t\t\t[\"oro\"] = 7,\r\n\t\t},\r\n\t},\r\n"
    ));
    assert!(lua.contains("[\"Goldberg Joestar\"] = \"Goldberg Joestar\","));
    assert!(lua.contains("[\"Otra Pj\"] = \"Otra Pj\","));
    assert!(lua.contains("[\"Otra Pj\"] = {\r\n[\"barras\"] = 5,\r\n},"));
    assert!(lua.ends_with("DominosVersion = 2\r\n"));
    assert!(!lua.contains("@@WOWPROFILE"));
    assert!(Documento::leer(lua.as_bytes()).unwrap().solo_datos());
    let personaje_c = m.raiz.join("WTF/Account/1#1/93/C");
    assert_eq!(
        std::fs::read_dir(personaje_c).unwrap().count(),
        0,
        "C no se toca"
    );
    assert_eq!(
        read(
            m.raiz
                .join("WTF/Account/1#1/93/B/SavedVariables/Dominos.lua")
        )
        .unwrap(),
        GENERICO_PJ
    );

    // Se deshace del último al primero.
    for id in ids.iter().rev() {
        let plan = m.operaciones.plan_deshacer(id).unwrap();
        m.operaciones.deshacer(id, &plan.huella, || false).unwrap();
    }
    let despues = instantanea(&m);
    let distintos: Vec<_> = antes
        .keys()
        .chain(despues.keys())
        .filter(|r| antes.get(*r) != despues.get(*r))
        .collect();
    assert!(distintos.is_empty(), "{distintos:?}");
}

#[test]
fn un_perfil_distinto_se_decide_en_la_vista_previa() {
    let m = mundo();
    let ruta = m.raiz.join("WTF/Account/1#1/SavedVariables/Dominos.lua");
    let con_perfil = CUENTA_LUA.replace(
        "\t\t[\"Default\"] = {",
        "\t\t[\"Goldberg Joestar\"] = {\r\n\t\t\t[\"barras\"] = 9,\r\n\t\t},\r\n\t\t[\"Default\"] = {",
    );
    write(&ruta, con_perfil).unwrap();
    let solo_a = |pedido: &Pedido| {
        m.abiertos
            .vista_previa(
                &m.sha256,
                vec![pj(&m, "A", "Goldberg Joestar")],
                pedido,
                false,
            )
            .unwrap()
    };
    let mut pedido = pedido(true);
    pedido.seleccion = vec!["addon-dominos-perfil".into()];
    let vista = solo_a(&pedido);
    let conflictos = &vista.plan.personajes[0].conflictos_perfil;
    assert_eq!(conflictos.len(), 1);
    assert_eq!(conflictos[0].propuesto, "Goldberg Joestar (2)");
    assert!(vista.plan.personajes[0].ficheros.is_empty());
    let sin_decidir = m.abiertos.aplicar(
        &m.sha256,
        &vista.huella,
        vec![pj(&m, "A", "Goldberg Joestar")],
        &pedido,
        &m.operaciones,
        || false,
    );
    assert!(
        sin_decidir.is_err(),
        "con un perfil sin decidir no se escribe"
    );

    pedido.perfiles = vec![DecisionPerfil {
        cuenta: CUENTA.into(),
        entrada: "addon-dominos-perfil".into(),
        accion: AccionPerfil::Renombrar,
    }];
    let vista = solo_a(&pedido);
    assert!(vista.plan.personajes[0].conflictos_perfil.is_empty());
    let fichero = &vista.plan.personajes[0].ficheros[0];
    let texto = String::from_utf8(fichero.contenido.clone()).unwrap();
    assert!(texto.contains("[\"Goldberg Joestar (2)\"] = {\r\n[\"barras\"] = 5,"));
    assert!(texto.contains("\t\t\t[\"barras\"] = 9,"));
}
