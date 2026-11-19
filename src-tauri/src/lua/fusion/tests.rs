use super::*;

const PJ: &str = "Goldberg Joestar";

fn rutas(lista: &[&str]) -> Vec<String> {
    lista.iter().map(|r| (*r).to_owned()).collect()
}

fn fusion(destino: &str, paquete: &str, seleccion: &Seleccion<'_>) -> Option<String> {
    let destino = Documento::leer(destino.as_bytes()).unwrap();
    let paquete = Documento::leer(paquete.as_bytes()).unwrap();
    fusionar_variable(&destino, &paquete, "Db", seleccion).map(|b| String::from_utf8(b).unwrap())
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

#[test]
fn incluir_cambia_lo_que_trae_y_deja_lo_demas_byte_a_byte() {
    let destino = "-- cabecera\r\nOtra = {\r\n\t[\"x\"] = 1,\r\n}\r\nDb = {\r\n[\"a\"] = 1,\r\n[\"b\"] = {\r\n[\"y\"] = 1,\r\n},\r\n[\"c\"] = 3,\r\n}\r\n";
    let paquete = "Db = {\n[\"a\"] = 2,\n[\"c\"] = 9,\n[\"d\"] = 4,\n}\n";
    let incluir = rutas(&["a", "b", "d"]);
    let salida = fusion(destino, paquete, &seleccion(Some(&incluir), &[], &[])).unwrap();
    assert!(salida.starts_with("-- cabecera\r\nOtra = {\r\n\t[\"x\"] = 1,\r\n}\r\nDb = "));
    // `c` no viaja aunque venga; `b` viaja pero no viene: se queda; `d` se añade.
    assert!(salida.contains("[\"a\"] = 2,"));
    assert!(salida.contains("[\"c\"] = 3,"));
    assert!(salida.contains("[\"b\"] = {\r\n[\"y\"] = 1,\r\n},"));
    assert!(salida.contains("[\"d\"] = 4,"));
    assert!(!salida.contains("= 9"));
    assert!(salida.ends_with("}\r\n"));
    assert!(Documento::leer(salida.as_bytes()).unwrap().solo_datos());
}

#[test]
fn sin_cambios_deja_el_fichero_igual() {
    let destino = "Db = {\n  [\"a\"] = 1, -- nota\n  [\"b\"] = {1, 2},\n}\n";
    let paquete = "Db = {\n[\"a\"] = 1,\n}\n";
    let incluir = rutas(&["a"]);
    assert_eq!(
        fusion(destino, paquete, &seleccion(Some(&incluir), &[], &[])).unwrap(),
        destino
    );
}

#[test]
fn entera_sustituye_la_variable() {
    let destino = "Db = {\n[\"viejo\"] = 1,\n}\nOtra = 2\n";
    let paquete = "Db = {\n[\"nuevo\"] = 1,\n}\n";
    assert_eq!(
        fusion(destino, paquete, &seleccion(None, &[], &[])).unwrap(),
        "Db = {\n[\"nuevo\"] = 1,\n}\nOtra = 2\n"
    );
}

#[test]
fn excluir_conserva_lo_excluido_del_destino() {
    let destino = "Db = {\n[\"historia\"] = \"del destino\",\n[\"op\"] = 1,\n}\n";
    let paquete = "Db = {\n[\"historia\"] = \"del paquete\",\n[\"op\"] = 2,\n}\n";
    let excluir = rutas(&["historia"]);
    let salida = fusion(destino, paquete, &seleccion(None, &excluir, &[])).unwrap();
    assert!(salida.contains("\"del destino\""));
    assert!(!salida.contains("\"del paquete\""));
    assert!(salida.contains("[\"op\"] = 2,"));
}

#[test]
fn solo_personaje_cambia_solo_la_entrada_del_destino() {
    let destino = concat!(
        "Db = {\n[\"Scales\"] = {\n[\"S1\"] = {\n[\"PerCharacterOptions\"] = {\n",
        "[\"Otro Pj\"] = {\n[\"Visible\"] = false,\n},\n",
        "[\"Goldberg Joestar\"] = {\n[\"Visible\"] = false,\n},\n},\n},\n},\n}\n"
    );
    let paquete = concat!(
        "Db = {\n[\"Scales\"] = {\n[\"S1\"] = {\n[\"PerCharacterOptions\"] = {\n",
        "[\"@@WOWPROFILE:PERSONAJE@@\"] = {\n[\"Visible\"] = true,\n},\n},\n},\n",
        "[\"S2\"] = {\n[\"PerCharacterOptions\"] = {\n[\"@@WOWPROFILE:PERSONAJE@@\"] = 1,\n},\n},\n},\n}\n"
    );
    let incluir = rutas(&["Scales"]);
    let solo = rutas(&["Scales/*/PerCharacterOptions/$PJ"]);
    let salida = fusion(destino, paquete, &seleccion(Some(&incluir), &[], &solo)).unwrap();
    assert!(salida.contains("[\"Otro Pj\"] = {\n[\"Visible\"] = false,\n},"));
    assert!(salida.contains("[\"Goldberg Joestar\"] = {\n[\"Visible\"] = true,\n},"));
    // Una escala que el destino no tiene llega con la clave del destino, nunca con el marcador.
    assert!(salida.contains(
        "[\"S2\"] = {\n[\"PerCharacterOptions\"] = {\n[\"Goldberg Joestar\"] = 1,\n},\n}"
    ));
    assert!(!salida.contains("@@WOWPROFILE"));
}

#[test]
fn una_variable_que_el_destino_no_tiene_se_anade_recortada() {
    let destino = "Otra = 1";
    let paquete = "Db = {\n[\"a\"] = 1,\n[\"secreto\"] = 2,\n}\n";
    let incluir = rutas(&["a"]);
    assert_eq!(
        fusion(destino, paquete, &seleccion(Some(&incluir), &[], &[])).unwrap(),
        "Otra = 1\nDb = {\n[\"a\"] = 1,\n}\n"
    );
    let nada = rutas(&["zz"]);
    assert_eq!(
        fusion(destino, paquete, &seleccion(Some(&nada), &[], &[])),
        None
    );
}

#[test]
fn listas_de_un_nivel_entero_vienen_del_paquete() {
    let destino = "Db = {\n[\"l\"] = {\n\"a\",\n[\"k\"] = 1,\n},\n}\n";
    let paquete = "Db = {\n[\"l\"] = {\n\"b\",\n\"c\",\n},\n}\n";
    let excluir = rutas(&["l/k"]);
    let salida = fusion(destino, paquete, &seleccion(None, &excluir, &[])).unwrap();
    assert_eq!(
        salida,
        "Db = {\n[\"l\"] = {\n\"b\",\n\"c\",\n[\"k\"] = 1,\n},\n}\n"
    );
}

#[test]
fn un_paquete_sin_lista_no_borra_la_del_destino() {
    let destino = "Db = {
[\"l\"] = {
\"a\",
[\"k\"] = 1,
[\"j\"] = 1,
},
}
";
    let paquete = "Db = {
[\"l\"] = {
[\"j\"] = 2,
},
}
";
    let excluir = rutas(&["l/k"]);
    let salida = fusion(destino, paquete, &seleccion(None, &excluir, &[])).unwrap();
    assert_eq!(
        salida,
        "Db = {
[\"l\"] = {
\"a\",
[\"k\"] = 1,
[\"j\"] = 2,
},
}
"
    );
}

fn perfil(destino: &str, pieza: &str, nombre: &str) -> String {
    let destino = Documento::leer(destino.as_bytes()).unwrap();
    let pieza = Documento::leer_valor(pieza.as_bytes()).unwrap();
    let salida = fusionar_perfil(
        &destino,
        &PerfilAcedb {
            variable: "DominosDB",
            pieza: &pieza,
            nombre,
            personaje: PJ,
        },
    )
    .unwrap();
    String::from_utf8(salida).unwrap()
}

const PIEZA: &str = concat!(
    "{\n[\"profile\"] = {\n[\"barras\"] = 5,\n},\n",
    "[\"namespaces\"] = {\n[\"Mod\"] = {\n[\"x\"] = 1,\n},\n},\n}\n"
);

#[test]
fn perfil_acedb_solo_toca_al_personaje_destino() {
    let destino = concat!(
        "DominosDB = {\n[\"profileKeys\"] = {\n[\"Otro Pj\"] = \"Default\",\n[\"Goldberg Joestar\"] = \"Default\",\n},\n",
        "[\"profiles\"] = {\n[\"Default\"] = {\n[\"barras\"] = 2,\n},\n},\n",
        "[\"char\"] = {\n[\"Otro Pj\"] = {\n[\"oro\"] = 1,\n},\n},\n}\n"
    );
    let salida = perfil(destino, PIEZA, "Importado");
    assert!(
        salida.contains("[\"Otro Pj\"] = \"Default\",\n[\"Goldberg Joestar\"] = \"Importado\",")
    );
    assert!(salida.contains(
        "[\"Default\"] = {\n[\"barras\"] = 2,\n},\n[\"Importado\"] = {\n[\"barras\"] = 5,\n},"
    ));
    assert!(salida.contains("[\"char\"] = {\n[\"Otro Pj\"] = {\n[\"oro\"] = 1,\n},\n},"));
    assert!(salida.contains(
        "[\"namespaces\"] = {\n[\"Mod\"] = {\n[\"profiles\"] = {\n[\"Importado\"] = {\n[\"x\"] = 1,\n},\n},\n},\n},"
    ));
    assert!(Documento::leer(salida.as_bytes()).unwrap().solo_datos());
}

#[test]
fn perfil_acedb_crea_la_variable_si_no_existe() {
    let salida = perfil("", PIEZA, "P");
    assert!(
        salida.starts_with(
            "DominosDB = {\n[\"profileKeys\"] = {\n[\"Goldberg Joestar\"] = \"P\",\n},"
        )
    );
    assert!(Documento::leer(salida.as_bytes()).unwrap().solo_datos());
}

#[test]
fn perfil_acedb_sobrescribe_el_perfil_y_el_del_modulo() {
    let destino = concat!(
        "DominosDB = {\n[\"profileKeys\"] = {\n},\n[\"profiles\"] = {\n[\"P\"] = {\n[\"barras\"] = 1,\n},\n},\n",
        "[\"namespaces\"] = {\n[\"Mod\"] = {\n[\"profiles\"] = {\n[\"P\"] = {\n[\"x\"] = 0,\n},\n},\n[\"char\"] = 7,\n},\n},\n}\n"
    );
    let salida = perfil(destino, PIEZA, "P");
    assert!(salida.contains("[\"P\"] = {\n[\"barras\"] = 5,\n}"));
    assert!(salida.contains("[\"P\"] = {\n[\"x\"] = 1,\n}"));
    assert!(salida.contains("[\"char\"] = 7,"));
    assert!(!salida.contains("= 0,"));
}

#[test]
fn una_pieza_sin_perfil_no_se_fusiona() {
    let destino = Documento::leer(b"").unwrap();
    let pieza = Documento::leer_valor(b"{\n[\"profile\"] = 1,\n}").unwrap();
    let perfil = PerfilAcedb {
        variable: "DominosDB",
        pieza: &pieza,
        nombre: "P",
        personaje: PJ,
    };
    assert!(fusionar_perfil(&destino, &perfil).is_none());
}
