//! Importar un paquete con carpetas de addons (#73, E2), de punta a punta: abrir, vista previa, aplicar y deshacer.

use super::super::{PaquetesAbiertos, Pedido};
use super::*;
use crate::deteccion::{self, PersonajeUbicado, Producto};
use crate::operaciones::Operaciones;
use crate::paquetes::{Ambito, CarpetaAddon, Categoria, Origen, Paquete, Perfil, Pieza, Tipo};
use std::fs::{create_dir_all, read_to_string, write};
use std::path::PathBuf;

const CUENTA: &str = "1#1";
const MACROS: &[u8] = b"VER 3 0100000000000001 \"Foco\" \"1\"\r\n/focus\r\nEND\r\n";

struct Mundo {
    _tmp: tempfile::TempDir,
    raiz: PathBuf,
    abiertos: PaquetesAbiertos,
    operaciones: Operaciones,
    sha256: String,
}

/// Forever con dos personajes, `Details` ya instalado en el destino, y abierto un paquete con macros y las carpetas de
/// `Dominos` y `Details`.
fn mundo() -> Mundo {
    let tmp = tempfile::tempdir().unwrap();
    let raiz = tmp.path().join("_forever_");
    write_all(
        &raiz.join(".flavor.info"),
        "Product Flavor!STRING:0\r\nwowf\r\n",
    );
    for pj in ["Goldberg", "Joestar"] {
        create_dir_all(raiz.join("WTF/Account").join(CUENTA).join("93").join(pj)).unwrap();
    }
    write_all(
        &deteccion::carpeta_addons(&raiz).join("Details/Details.toc"),
        "## Title: Details del destino",
    );
    let origen = tmp.path().join("origen");
    for addon in ["Dominos", "Details"] {
        write_all(
            &origen.join(addon).join(format!("{addon}.toc")),
            &format!("## Title: {addon}"),
        );
        write_all(&origen.join(addon).join("Libs/lib.lua"), "local x = 1");
    }
    let fichero = tmp.path().join("p.wowprofile");
    let macros = Pieza {
        id: "macros".into(),
        categoria: Categoria::Macros,
        tipo: Tipo::Fichero,
        ambito: Ambito::Personaje,
        destino: "personaje/macros-cache.txt".into(),
        contenido: MACROS.to_vec(),
        quitadas: Vec::new(),
        addon: None,
    };
    let carpeta = |addon: &str| CarpetaAddon {
        id: format!("carpeta-{}", addon.to_lowercase()),
        carpeta: addon.into(),
        origen: origen.join(addon),
    };
    Paquete::nuevo(perfil(), origen_paquete(), Vec::new(), vec![macros])
        .unwrap()
        .con_carpetas(vec![carpeta("Dominos"), carpeta("Details")])
        .unwrap()
        .guardar(&fichero)
        .unwrap();
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

fn write_all(ruta: &std::path::Path, texto: &str) {
    create_dir_all(ruta.parent().unwrap()).unwrap();
    write(ruta, texto).unwrap();
}

fn perfil() -> Perfil {
    Perfil {
        nombre: "Druida".into(),
        descripcion: None,
        creado: "2026-11-19T10:00:00Z".into(),
    }
}

fn origen_paquete() -> Origen {
    Origen {
        producto: "wowf",
        build: "1.60.1.70938".into(),
        interfaz: 16001,
    }
}

fn pjs(m: &Mundo) -> Vec<PersonajeUbicado> {
    ["Goldberg", "Joestar"]
        .iter()
        .map(|carpeta| PersonajeUbicado {
            raiz_version: m.raiz.clone(),
            producto: Producto::Forever,
            cuenta: CUENTA.into(),
            reino: "93".into(),
            personaje: (*carpeta).into(),
            nombre_en_datos: None,
        })
        .collect()
}

fn pedido() -> Pedido {
    Pedido {
        seleccion: vec![
            "macros".into(),
            "carpeta-dominos".into(),
            "carpeta-details".into(),
        ],
        cuentas: Vec::new(),
        perfiles: Vec::new(),
    }
}

fn addons(m: &Mundo) -> PathBuf {
    deteccion::carpeta_addons(&m.raiz)
}

#[test]
fn instala_solo_los_addons_que_no_estan_y_una_vez_para_toda_la_instalacion() {
    let m = mundo();
    let resumen = m
        .abiertos
        .abrir(&m._tmp.path().join("p.wowprofile"))
        .unwrap();
    let carpeta = resumen
        .entradas
        .iter()
        .find(|e| e.id == "carpeta-dominos")
        .unwrap();
    assert!(carpeta.marcada && carpeta.incompatible.is_none());

    let vista = m
        .abiertos
        .vista_previa(&m.sha256, pjs(&m), &pedido(), false)
        .unwrap();

    let plan = &vista.plan.personajes;
    let estados: Vec<_> = plan[0]
        .addons
        .iter()
        .map(|a| (a.addon.as_str(), a.ya_instalado))
        .collect();
    assert_eq!(estados, [("Dominos", false), ("Details", true)]);
    assert!(plan[1].addons.is_empty(), "solo en el primer personaje");

    let resultados = m
        .abiertos
        .aplicar(
            &m.sha256,
            &vista.huella,
            pjs(&m),
            &pedido(),
            &m.operaciones,
            || false,
        )
        .unwrap();

    assert!(matches!(
        resultados[0].resultado,
        Resultado::Completa { addons: 1, .. }
    ));
    assert!(matches!(
        resultados[1].resultado,
        Resultado::Completa { addons: 0, .. }
    ));
    assert_eq!(
        read_to_string(addons(&m).join("Dominos/Libs/lib.lua")).unwrap(),
        "local x = 1"
    );
    assert_eq!(
        read_to_string(addons(&m).join("Details/Details.toc")).unwrap(),
        "## Title: Details del destino",
        "el instalado no se toca"
    );

    let Resultado::Completa { operacion, .. } = &resultados[0].resultado else {
        unreachable!()
    };
    let deshacer = m.operaciones.plan_deshacer(operacion).unwrap();
    m.operaciones
        .deshacer(operacion, &deshacer.huella, || false)
        .unwrap();
    assert!(!addons(&m).join("Dominos").exists());
    assert!(addons(&m).join("Details/Details.toc").exists());
}

#[test]
fn si_el_addon_aparece_entre_la_vista_previa_y_aplicar_no_se_aplica_nada() {
    let m = mundo();
    let vista = m
        .abiertos
        .vista_previa(&m.sha256, pjs(&m), &pedido(), false)
        .unwrap();
    write_all(&addons(&m).join("dominos/otro.toc"), "otro");

    let resultado = m.abiertos.aplicar(
        &m.sha256,
        &vista.huella,
        pjs(&m),
        &pedido(),
        &m.operaciones,
        || false,
    );

    assert_eq!(resultado, Err(ErrorNucleo::PlanCaducado {}));
    assert_eq!(
        read_to_string(addons(&m).join("dominos/otro.toc")).unwrap(),
        "otro"
    );
}
