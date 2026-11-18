use super::super::{DecisionCuenta, PaquetesAbiertos, Pedido};
use super::*;
use crate::deteccion::{PersonajeUbicado, Producto};
use crate::operaciones::Operaciones;
use crate::paquetes::{Ambito, Categoria, Origen, Paquete, Perfil, Pieza, Tipo};
use std::fs::{create_dir_all, read, write};
use std::path::{Path, PathBuf};

const MACROS: &[u8] =
    b"VER 3 0100000000000001 \"Foco\" \"1\"\r\n/cast [@focus] Maleficio\r\nEND\r\n";
const MACROS_CUENTA: &[u8] = b"VER 3 0000000000000002 \"RS\" \"1\"\r\n/rs\r\nEND\r\n";
const VIEJAS: &[u8] = b"VER 3 0100000000000001 \"Viejo\" \"1\"\r\n/say hola\r\nEND\r\n";
const CUENTA: &str = "1#1";

struct Mundo {
    _tmp: tempfile::TempDir,
    raiz: PathBuf,
    fichero: PathBuf,
    datos: PathBuf,
    abiertos: PaquetesAbiertos,
    operaciones: Operaciones,
    sha256: String,
}

fn pieza(id: &'static str, ambito: Ambito, destino: &'static str, contenido: &[u8]) -> Pieza {
    Pieza {
        id,
        categoria: Categoria::Macros,
        tipo: Tipo::Fichero,
        ambito,
        destino,
        contenido: contenido.to_vec(),
        claves_quitadas: Vec::new(),
    }
}

fn guardar_paquete(fichero: &Path, macros: &[u8]) {
    let perfil = Perfil {
        nombre: "Druida".into(),
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
            "macros",
            Ambito::Personaje,
            "personaje/macros-cache.txt",
            macros,
        ),
        pieza(
            "macros-cuenta",
            Ambito::Cuenta,
            "cuenta/macros-cache.txt",
            MACROS_CUENTA,
        ),
    ];
    Paquete::nuevo(perfil, origen, Vec::new(), piezas)
        .unwrap()
        .guardar(fichero)
        .unwrap();
}

/// Una instalación de Forever con esos personajes en la cuenta `1#1`, cada uno con sus macros viejas, y el paquete
/// abierto.
fn mundo(personajes: &[&str]) -> Mundo {
    let tmp = tempfile::tempdir().unwrap();
    let raiz = tmp.path().join("_forever_");
    create_dir_all(raiz.join("Interface/AddOns")).unwrap();
    write(
        raiz.join(".flavor.info"),
        "Product Flavor!STRING:0\r\nwowf\r\n",
    )
    .unwrap();
    for carpeta in personajes {
        let pj = raiz
            .join("WTF/Account")
            .join(CUENTA)
            .join("93")
            .join(carpeta);
        create_dir_all(&pj).unwrap();
        write(pj.join("macros-cache.txt"), VIEJAS).unwrap();
    }
    let fichero = tmp.path().join("druida.wowprofile");
    guardar_paquete(&fichero, MACROS);
    let abiertos = PaquetesAbiertos::default();
    let sha256 = abiertos.abrir(&fichero).unwrap().sha256;
    let datos = tmp.path().join("operaciones");
    Mundo {
        operaciones: Operaciones::en(datos.clone()),
        _tmp: tmp,
        raiz,
        fichero,
        datos,
        abiertos,
        sha256,
    }
}

fn pj(m: &Mundo, carpeta: &str) -> PersonajeUbicado {
    PersonajeUbicado {
        raiz_version: m.raiz.clone(),
        producto: Producto::Forever,
        cuenta: CUENTA.into(),
        reino: "93".into(),
        personaje: carpeta.into(),
        nombre_en_datos: None,
    }
}

fn pjs(m: &Mundo, carpetas: &[&str]) -> Vec<PersonajeUbicado> {
    carpetas.iter().map(|c| pj(m, c)).collect()
}

fn pedido(decidido: bool) -> Pedido {
    Pedido {
        seleccion: vec!["macros".into(), "macros-cuenta".into()],
        cuentas: if decidido {
            vec![DecisionCuenta {
                cuenta: CUENTA.into(),
                entrada: "macros-cuenta".into(),
                aceptar: true,
            }]
        } else {
            Vec::new()
        },
    }
}

fn huella_vista(m: &Mundo, carpetas: &[&str], pedido: &Pedido) -> String {
    m.abiertos
        .vista_previa(&m.sha256, pjs(m, carpetas), pedido, false)
        .unwrap()
        .huella
}

fn aplicar_en(
    m: &Mundo,
    carpetas: &[&str],
    huella: &str,
    pedido: &Pedido,
) -> Result<Vec<ResultadoPersonaje>, ErrorNucleo> {
    m.abiertos.aplicar(
        &m.sha256,
        huella,
        pjs(m, carpetas),
        pedido,
        &m.operaciones,
        || false,
    )
}

fn operaciones_creadas(m: &Mundo) -> usize {
    std::fs::read_dir(&m.datos).map_or(0, |d| d.count())
}

fn macros_de(m: &Mundo, carpeta: &str) -> Vec<u8> {
    read(pj(m, carpeta).carpeta().join("macros-cache.txt")).unwrap()
}

#[test]
fn una_operacion_por_personaje_y_lo_de_cuenta_en_el_primero() {
    let m = mundo(&["A", "B"]);
    let huella = huella_vista(&m, &["A", "B"], &pedido(true));

    let resultados = aplicar_en(&m, &["A", "B"], &huella, &pedido(true)).unwrap();

    let ficheros: Vec<_> = resultados
        .iter()
        .map(|r| match &r.resultado {
            Resultado::Completa { ficheros, .. } => *ficheros,
            otro => panic!("{otro:?}"),
        })
        .collect();
    assert_eq!(ficheros, [2, 1]);
    assert_eq!(macros_de(&m, "A"), MACROS);
    assert_eq!(macros_de(&m, "B"), MACROS);
    let cuenta = pj(&m, "A").carpeta_cuenta().join("macros-cache.txt");
    assert_eq!(read(cuenta).unwrap(), MACROS_CUENTA);
    assert_eq!(operaciones_creadas(&m), 2);
    let json = serde_json::to_value(&resultados[0]).unwrap();
    assert_eq!(json["resultado"], "completa");
    assert_eq!(json["personaje"], "A");
    assert!(json["operacion"].is_string());
}

#[test]
fn un_personaje_sin_nada_que_escribir_no_crea_operacion() {
    let m = mundo(&["A"]);
    write(pj(&m, "A").carpeta().join("macros-cache.txt"), MACROS).unwrap();
    let solo_pj = Pedido {
        seleccion: vec!["macros".into()],
        cuentas: Vec::new(),
    };
    let huella = huella_vista(&m, &["A"], &solo_pj);

    let resultados = aplicar_en(&m, &["A"], &huella, &solo_pj).unwrap();

    assert_eq!(resultados[0].resultado, Resultado::SinCambios);
    assert_eq!(operaciones_creadas(&m), 0);
}

#[test]
fn si_el_disco_cambia_tras_la_vista_previa_no_escribe_nada() {
    let m = mundo(&["A", "B"]);
    let huella = huella_vista(&m, &["A", "B"], &pedido(true));
    write(pj(&m, "B").carpeta().join("macros-cache.txt"), b"del juego").unwrap();

    let resultado = aplicar_en(&m, &["A", "B"], &huella, &pedido(true));

    assert_eq!(resultado, Err(ErrorNucleo::PlanCaducado {}));
    assert_eq!(macros_de(&m, "A"), VIEJAS);
    assert_eq!(operaciones_creadas(&m), 0);
}

#[test]
fn si_el_paquete_cambio_o_ya_no_esta_no_se_aplica() {
    let m = mundo(&["A"]);
    let huella = huella_vista(&m, &["A"], &pedido(true));
    guardar_paquete(&m.fichero, b"VER 3 otro\r\n");

    assert_eq!(
        aplicar_en(&m, &["A"], &huella, &pedido(true)),
        Err(ErrorNucleo::PaqueteCambiado {})
    );
    std::fs::remove_file(&m.fichero).unwrap();
    assert_eq!(
        aplicar_en(&m, &["A"], &huella, &pedido(true)),
        Err(ErrorNucleo::PaqueteCambiado {})
    );
    assert_eq!(macros_de(&m, "A"), VIEJAS);
    assert_eq!(operaciones_creadas(&m), 0);
}

#[test]
fn con_algo_sin_decidir_no_se_aplica() {
    let m = mundo(&["A"]);
    let huella = huella_vista(&m, &["A"], &pedido(false));

    assert_eq!(
        aplicar_en(&m, &["A"], &huella, &pedido(false)),
        Err(ErrorNucleo::ConflictosSinDecidir {})
    );
    assert_eq!(operaciones_creadas(&m), 0);
}

/// Las macros de B están abiertas por otro programa sin dejar escribir: A queda aplicado, B restaurado y C sin tocar.
#[test]
#[cfg(windows)]
fn si_falla_uno_los_anteriores_quedan_y_los_siguientes_no_se_aplican() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;

    let m = mundo(&["A", "B", "C"]);
    let todos = ["A", "B", "C"];
    let huella = huella_vista(&m, &todos, &pedido(true));
    let abierto = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(pj(&m, "B").carpeta().join("macros-cache.txt"))
        .unwrap();

    let resultados = aplicar_en(&m, &todos, &huella, &pedido(true)).unwrap();
    drop(abierto);

    assert!(matches!(
        resultados[0].resultado,
        Resultado::Completa { .. }
    ));
    assert!(
        matches!(
            &resultados[1].resultado,
            Resultado::Fallida {
                error: ErrorNucleo::OperacionFallida {
                    restaurada: true,
                    ..
                }
            }
        ),
        "{:?}",
        resultados[1]
    );
    assert_eq!(resultados[2].resultado, Resultado::NoAplicada);
    assert_eq!(macros_de(&m, "A"), MACROS);
    assert_eq!(macros_de(&m, "B"), VIEJAS);
    assert_eq!(macros_de(&m, "C"), VIEJAS);
    assert_eq!(operaciones_creadas(&m), 2);
}
