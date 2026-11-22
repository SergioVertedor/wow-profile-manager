//! Importar el Modo Edición (#71): el diseño se añade a la cuenta y el personaje pasa a usarlo.

use super::*;
use crate::deteccion::Producto;
use serde_json::{Value, json};
use std::path::Path;

/// Un diseño `Sanador` con dos marcos, en un fichero de cuenta como el que viaja en el paquete.
const PAQUETE: &str =
    "5 1 0 7 Sanador 0 2 0 0 1 7 7 UIParent 0.0 -4.0 -1 ## 6 0 0 4 4 UIParent 1.0 2.0 -1 #$\0";
/// La cuenta del destino, con su diseño `default` (otros marcos).
const CUENTA: &str =
    "5 1 0 7 default 0 2 0 0 1 7 7 UIParent 0.0 -4.0 -1 ## 6 0 0 4 4 UIParent 9.0 9.0 -1 #$\0";
const USA_DEFAULT: &str = "5 3 3 3 3 3 -1 -1 -1 -1 -1\0";
const ID: &str = "modo-edicion";
const DESTINO_CUENTA: &str = "cuenta/edit-mode-cache-account.txt";

fn entrada(tipo: &str, adaptador: Option<Value>) -> Value {
    let mut entrada = json!({
        "id": ID, "categoria": "modo-edicion", "tipo": tipo, "ambito": "cuenta",
        "destino": DESTINO_CUENTA, "sha256": "0".repeat(64), "bytes": 0, "perfil": "Sanador"
    });
    if let Some(adaptador) = adaptador {
        entrada["adaptador"] = adaptador;
    }
    entrada
}

fn paquete_con(entrada: Value) -> PaqueteLeido {
    let manifiesto = json!({
        "formato": { "nombre": "wowprofile", "version": "1.0" },
        "perfil": { "nombre": "Sanador", "creado": "2026-11-19T10:00:00Z" },
        "origen": { "producto": "wowf", "build": "1.60.1.70959", "interfaz": 16001 },
        "addons": [],
        "entradas": [entrada]
    });
    PaqueteLeido::de_prueba(
        serde_json::from_value(manifiesto).unwrap(),
        &[(ID, PAQUETE.as_bytes())],
    )
}

fn paquete() -> PaqueteLeido {
    paquete_con(entrada(
        "adaptador",
        Some(json!({ "id": "modo-edicion", "version": 1 })),
    ))
}

fn destino(cuenta: &str, carpeta: &str, actuales: &[(&str, &str)]) -> Destino {
    Destino {
        personaje: PersonajeUbicado {
            raiz_version: Path::new("C:/wow").join("_forever_"),
            producto: Producto::Forever,
            cuenta: cuenta.into(),
            reino: "93".into(),
            personaje: carpeta.into(),
            nombre_en_datos: None,
        },
        actuales: actuales
            .iter()
            .map(|(d, t)| ((*d).to_owned(), Some(t.as_bytes().to_vec())))
            .collect(),
        instalados: HashSet::new(),
        en_addons: HashSet::new(),
        versiones: HashMap::new(),
    }
}

fn pedido() -> Pedido {
    Pedido {
        seleccion: vec![ID.to_owned()],
        cuentas: Vec::new(),
        perfiles: Vec::new(),
    }
}

fn fichero<'p>(plan: &'p PlanPersonaje, destino: &str) -> &'p FicheroPlan {
    plan.ficheros.iter().find(|f| f.destino == destino).unwrap()
}

#[test]
fn anade_el_diseno_a_la_cuenta_y_el_personaje_pasa_a_usarlo_sin_preguntar_por_la_cuenta() {
    let destinos = [destino(
        "1#1",
        "Ana",
        &[
            (DESTINO_CUENTA, CUENTA),
            (modo_edicion::DESTINO_PERSONAJE, USA_DEFAULT),
        ],
    )];

    let plan = planificar(&paquete(), &destinos, &pedido()).unwrap();

    let ana = &plan.personajes[0];
    assert!(ana.conflictos.is_empty(), "no pide decidir la cuenta");
    let cuenta = fichero(ana, DESTINO_CUENTA);
    assert_eq!(
        (cuenta.grupo, cuenta.accion, cuenta.de_cuenta),
        (Grupo::Modificado, Accion::Fusionar, false)
    );
    assert!(
        cuenta
            .contenido
            .starts_with(CUENTA.trim_end_matches('\0').as_bytes())
    );
    assert_eq!(cuenta.elementos[0].clase, Some(Clase::Diseno));
    assert_eq!(cuenta.elementos[0].nombre, "Sanador");
    let personaje = fichero(ana, modo_edicion::DESTINO_PERSONAJE);
    assert_eq!(personaje.contenido, b"5 4 3 3 3 3 -1 -1 -1 -1 -1\0");
    assert_eq!(personaje.elementos[0].clase, Some(Clase::DisenoActivo));
    assert!(!personaje.en_cuenta);
}

#[test]
fn en_la_misma_cuenta_solo_el_primero_escribe_el_fichero_de_cuenta() {
    let actuales = [
        (DESTINO_CUENTA, CUENTA),
        (modo_edicion::DESTINO_PERSONAJE, USA_DEFAULT),
    ];
    let destinos = [
        destino("1#1", "Ana", &actuales),
        destino("1#1", "Bruno", &actuales),
        destino("2#1", "Carla", &actuales),
    ];

    let plan = planificar(&paquete(), &destinos, &pedido()).unwrap();

    let grupos: Vec<_> = plan
        .personajes
        .iter()
        .map(|p| {
            (
                fichero(p, DESTINO_CUENTA).grupo,
                fichero(p, modo_edicion::DESTINO_PERSONAJE).grupo,
            )
        })
        .collect();
    assert_eq!(
        grupos,
        [
            (Grupo::Modificado, Grupo::Modificado),
            (Grupo::SinCambios, Grupo::Modificado),
            (Grupo::Modificado, Grupo::Modificado),
        ]
    );
    assert_eq!(
        fichero(&plan.personajes[1], DESTINO_CUENTA)
            .escrita_en
            .as_deref(),
        Some("Ana")
    );
    assert!(
        fichero(&plan.personajes[1], DESTINO_CUENTA)
            .elementos
            .is_empty(),
        "no se enseña dos veces"
    );
    // Los dos usan el mismo diseño: el que añade el primero.
    assert_eq!(
        fichero(&plan.personajes[1], modo_edicion::DESTINO_PERSONAJE).contenido,
        b"5 4 3 3 3 3 -1 -1 -1 -1 -1\0"
    );
}

#[test]
fn sin_ficheros_en_el_destino_se_crean_los_dos() {
    let plan = planificar(&paquete(), &[destino("1#1", "Ana", &[])], &pedido()).unwrap();

    let ana = &plan.personajes[0];
    assert_eq!(fichero(ana, DESTINO_CUENTA).accion, Accion::Crear);
    assert_eq!(fichero(ana, DESTINO_CUENTA).contenido, PAQUETE.as_bytes());
    assert_eq!(
        fichero(ana, modo_edicion::DESTINO_PERSONAJE).contenido,
        b"5 3 3 3 3 3 -1 -1 -1 -1 -1\0"
    );
}

#[test]
fn reimportar_el_mismo_diseno_no_cambia_nada() {
    let ya_importado = format!(
        "{} 7 Sanador 0 2 0 0 1 7 7 UIParent 0.0 -4.0 -1 ## 6 0 0 4 4 UIParent 1.0 2.0 -1 #$\0",
        CUENTA.trim_end_matches('\0')
    );
    let destinos = [destino(
        "1#1",
        "Ana",
        &[
            (DESTINO_CUENTA, &ya_importado),
            (
                modo_edicion::DESTINO_PERSONAJE,
                "5 4 3 3 3 3 -1 -1 -1 -1 -1\0",
            ),
        ],
    )];

    let plan = planificar(&paquete(), &destinos, &pedido()).unwrap();

    assert!(
        plan.personajes[0]
            .ficheros
            .iter()
            .all(|f| f.grupo == Grupo::SinCambios && f.elementos.is_empty())
    );
}

#[test]
fn un_destino_que_no_se_entiende_no_se_toca() {
    let destinos = [destino("1#1", "Ana", &[(DESTINO_CUENTA, "6 0\0")])];

    let plan = planificar(&paquete(), &destinos, &pedido()).unwrap();

    let ana = &plan.personajes[0];
    assert!(ana.ficheros.is_empty());
    assert_eq!(
        ana.no_aplicables,
        [NoAplicable {
            entrada: ID.to_owned(),
            motivo: MotivoNoAplicable::DestinoIlegible
        }]
    );
}

#[test]
fn solo_se_importa_con_su_adaptador() {
    let con = |entrada| {
        let paquete = paquete_con(entrada);
        clasificar(&paquete.manifiesto.entradas[0], &paquete).err()
    };

    assert_eq!(
        con(entrada("fichero", None)),
        Some(MotivoIncompatible::NoSeImporta)
    );
    assert_eq!(
        con(entrada(
            "adaptador",
            Some(json!({ "id": "otro", "version": 1 }))
        )),
        Some(MotivoIncompatible::AdaptadorDesconocido)
    );
    assert_eq!(
        con(entrada(
            "adaptador",
            Some(json!({ "id": "modo-edicion", "version": 2 }))
        )),
        Some(MotivoIncompatible::AdaptadorDesconocido)
    );
    assert_eq!(
        con(entrada(
            "adaptador",
            Some(json!({ "id": "modo-edicion", "version": 1 }))
        )),
        None
    );
}

#[test]
fn en_el_resumen_no_cuenta_como_entrada_de_cuenta() {
    let resumen = resumen(&paquete());

    let entrada = &resumen.entradas[0];
    assert!(!entrada.de_cuenta, "no hay que decidirlo por cuenta");
    assert!(entrada.marcada && entrada.incompatible.is_none());
    assert_eq!(entrada.perfil.as_deref(), Some("Sanador"));
}
