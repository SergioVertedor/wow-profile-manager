use super::*;
use crate::lua::MARCADOR_PERSONAJE;
use serde_json::{Value, json};

const PJ: &str = "Goldberg Joestar";

fn entrada(tipo: &str, destino: &str, extra: Value) -> EntradaLeida {
    let mut valor = json!({
        "id": "addon-x", "categoria": "config-addon", "tipo": tipo,
        "ambito": if destino.starts_with("cuenta/") { "cuenta" } else { "personaje" },
        "destino": destino, "addon": "Dominos", "sha256": "0".repeat(64), "bytes": 0
    });
    if let (Value::Object(base), Value::Object(mas)) = (&mut valor, extra) {
        base.extend(mas);
    }
    serde_json::from_value(valor).unwrap()
}

fn perfil_acedb(perfil: &str) -> EntradaLeida {
    entrada(
        "perfil-acedb",
        "cuenta/SavedVariables/Dominos.lua",
        json!({ "variable": "DominosDB", "perfil": perfil }),
    )
}

fn pawn() -> EntradaLeida {
    entrada(
        "adaptador",
        "cuenta/SavedVariables/Pawn.lua",
        json!({ "addon": "Pawn", "variable": "PawnCommon", "adaptador": { "id": "pawn", "version": 2 } }),
    )
}

const PIEZA_PERFIL: &[u8] = b"{\n[\"profile\"] = {\n[\"barras\"] = 5,\n},\n}\n";

fn hecha(aplicacion: Aplicacion) -> (String, Vec<Elemento>) {
    match aplicacion {
        Aplicacion::Hecha {
            contenido,
            elementos,
        } => (String::from_utf8(contenido).unwrap(), elementos),
        otra => panic!("{otra:?}"),
    }
}

#[test]
fn clasifica_cada_tipo_bueno() {
    let perfil = perfil_acedb("Default");
    let addon = clasificar(&perfil, Tipo::PerfilAcedb, PIEZA_PERFIL).unwrap();
    assert!(addon.en_cuenta && addon.marcada() && !addon.afecta_cuenta());
    assert_eq!(addon.fichero, "SavedVariables/Dominos.lua");

    let pawn = pawn();
    let addon = clasificar(&pawn, Tipo::Adaptador, b"PawnCommon = {\n}\n").unwrap();
    assert!(addon.marcada() && addon.afecta_cuenta());

    let generico = entrada("fichero", "personaje/SavedVariables/Dominos.lua", json!({}));
    let addon = clasificar(&generico, Tipo::Fichero, b"A = 1\nB = { \"x\" }\n").unwrap();
    assert!(!addon.marcada() && !addon.afecta_cuenta() && !addon.en_cuenta);
}

#[test]
fn rechaza_codigo_destino_ajeno_y_adaptadores_que_no_conoce() {
    let generico = entrada("fichero", "personaje/SavedVariables/Dominos.lua", json!({}));
    let motivo = |e: &EntradaLeida, tipo, pieza: &[u8]| clasificar(e, tipo, pieza).unwrap_err();
    assert_eq!(
        motivo(&generico, Tipo::Fichero, b"A = RunScript(\"x\")\n"),
        MotivoIncompatible::PiezaNoValida
    );
    let otra_carpeta = entrada("fichero", "personaje/SavedVariables/Otro.lua", json!({}));
    assert_eq!(
        motivo(&otra_carpeta, Tipo::Fichero, b"A = 1"),
        MotivoIncompatible::DestinoAjeno
    );
    let escapa = entrada(
        "fichero",
        "personaje/SavedVariables/../x.lua",
        json!({ "addon": ".." }),
    );
    assert_eq!(
        motivo(&escapa, Tipo::Fichero, b"A = 1"),
        MotivoIncompatible::DestinoAjeno
    );
    let perfil_de_personaje = entrada(
        "perfil-acedb",
        "personaje/SavedVariables/Dominos.lua",
        json!({ "variable": "DominosDB", "perfil": "P" }),
    );
    assert_eq!(
        motivo(&perfil_de_personaje, Tipo::PerfilAcedb, PIEZA_PERFIL),
        MotivoIncompatible::DestinoAjeno
    );
    assert_eq!(
        motivo(
            &perfil_acedb("P"),
            Tipo::PerfilAcedb,
            b"{ [\"profile\"] = 1 }"
        ),
        MotivoIncompatible::PiezaNoValida
    );
    let reservada = entrada(
        "perfil-acedb",
        "cuenta/SavedVariables/Dominos.lua",
        json!({ "variable": "end", "perfil": "P" }),
    );
    assert_eq!(
        motivo(&reservada, Tipo::PerfilAcedb, PIEZA_PERFIL),
        MotivoIncompatible::PiezaNoValida
    );
    let mas_nuevo = entrada(
        "adaptador",
        "cuenta/SavedVariables/Pawn.lua",
        json!({ "addon": "Pawn", "variable": "PawnCommon", "adaptador": { "id": "pawn", "version": 99 } }),
    );
    let sin_regla = entrada(
        "adaptador",
        "cuenta/SavedVariables/Pawn.lua",
        json!({ "addon": "Pawn", "variable": "PawnSecreto", "adaptador": { "id": "pawn", "version": 2 } }),
    );
    for e in [&mas_nuevo, &sin_regla] {
        assert_eq!(
            motivo(e, Tipo::Adaptador, b"PawnCommon = {}"),
            MotivoIncompatible::AdaptadorDesconocido
        );
    }
    assert_eq!(
        motivo(&pawn(), Tipo::Adaptador, b"PawnCommon = {}\nOtra = 1"),
        MotivoIncompatible::PiezaNoValida
    );
}

#[test]
fn perfil_con_marcador_se_llama_como_el_destino_y_solo_cambia_su_clave() {
    let e = perfil_acedb(MARCADOR_PERSONAJE);
    let addon = clasificar(&e, Tipo::PerfilAcedb, PIEZA_PERFIL).unwrap();
    let destino = "DominosDB = {\n[\"profileKeys\"] = {\n[\"Otro Pj\"] = \"Default\",\n},\n[\"profiles\"] = {\n},\n}\n";
    let (contenido, elementos) = hecha(aplicar(
        &addon,
        PIEZA_PERFIL,
        destino.as_bytes(),
        Some(PJ),
        None,
    ));
    assert!(
        contenido.contains(
            "[\"Otro Pj\"] = \"Default\",\n[\"Goldberg Joestar\"] = \"Goldberg Joestar\","
        )
    );
    assert!(contenido.contains("[\"Goldberg Joestar\"] = {\n[\"barras\"] = 5,\n}"));
    assert_eq!(elementos.len(), 2);
    assert_eq!(
        (elementos[0].clase, elementos[0].cambio),
        (Some(Clase::Perfil), Cambio::Anadido)
    );
    assert_eq!(elementos[1].clase, Some(Clase::PerfilDe));
}

#[test]
fn un_perfil_distinto_es_conflicto_hasta_decidir() {
    let e = perfil_acedb("Default");
    let addon = clasificar(&e, Tipo::PerfilAcedb, PIEZA_PERFIL).unwrap();
    let destino = concat!(
        "DominosDB = {\n[\"profileKeys\"] = {\n[\"Otro Pj\"] = \"Default\",\n[\"Goldberg Joestar\"] = \"Default\",\n},\n",
        "[\"profiles\"] = {\n[\"Default\"] = {\n[\"barras\"] = 1,\n},\n[\"Default (2)\"] = {\n},\n},\n}\n"
    );
    let aplicar_con =
        |decision| aplicar(&addon, PIEZA_PERFIL, destino.as_bytes(), Some(PJ), decision);
    let Aplicacion::Conflicto(conflicto) = aplicar_con(None) else {
        panic!("tenía que ser conflicto");
    };
    assert_eq!(conflicto.perfil, "Default");
    assert_eq!(conflicto.propuesto, "Default (3)");
    assert_eq!(conflicto.usado_por, ["Otro Pj"]);

    let (renombrado, _) = hecha(aplicar_con(Some(AccionPerfil::Renombrar)));
    assert!(renombrado.contains("[\"Default\"] = {\n[\"barras\"] = 1,\n}"));
    assert!(renombrado.contains("[\"Default (3)\"] = {\n[\"barras\"] = 5,\n}"));
    assert!(renombrado.contains("[\"Otro Pj\"] = \"Default\""));
    assert!(renombrado.contains("[\"Goldberg Joestar\"] = \"Default (3)\""));

    let (sobrescrito, elementos) = hecha(aplicar_con(Some(AccionPerfil::Sobrescribir)));
    assert!(sobrescrito.contains("[\"Default\"] = {\n[\"barras\"] = 5,\n}"));
    assert_eq!(elementos[0].cambio, Cambio::Cambiado);
}

#[test]
fn un_perfil_igual_no_es_conflicto() {
    let e = perfil_acedb("Default");
    let addon = clasificar(&e, Tipo::PerfilAcedb, PIEZA_PERFIL).unwrap();
    let destino = "DominosDB = {\n[\"profileKeys\"] = {\n},\n[\"profiles\"] = {\n[\"Default\"] = {\n[\"barras\"] = 5,\n},\n},\n}\n";
    let (_, elementos) = hecha(aplicar(
        &addon,
        PIEZA_PERFIL,
        destino.as_bytes(),
        Some(PJ),
        None,
    ));
    assert_eq!(elementos.len(), 1, "solo cambia a qué perfil apunta");
    assert_eq!(elementos[0].clase, Some(Clase::PerfilDe));
}

#[test]
fn sin_clave_o_con_un_destino_que_no_se_entiende_no_se_aplica() {
    let e = perfil_acedb("Default");
    let addon = clasificar(&e, Tipo::PerfilAcedb, PIEZA_PERFIL).unwrap();
    assert_eq!(
        aplicar(&addon, PIEZA_PERFIL, b"", None, None),
        Aplicacion::NoAplicable(MotivoNoAplicable::SinNombre)
    );
    assert_eq!(
        aplicar(&addon, PIEZA_PERFIL, b"DominosDB = {", Some(PJ), None),
        Aplicacion::NoAplicable(MotivoNoAplicable::DestinoIlegible)
    );
}

#[test]
fn adaptador_aplica_la_regla_de_la_app_y_la_clave_del_destino() {
    let e = pawn();
    // La pieza trae una clave que la regla de Pawn no deja pasar (`Secreto`) y la entrada del personaje con marcador.
    let pieza = concat!(
        "PawnCommon = {\n[\"Digits\"] = 2,\n[\"Secreto\"] = 1,\n[\"Scales\"] = {\n[\"S\"] = {\n",
        "[\"PerCharacterOptions\"] = {\n[\"@@WOWPROFILE:PERSONAJE@@\"] = {\n[\"Visible\"] = true,\n},\n},\n},\n},\n}\n"
    );
    let addon = clasificar(&e, Tipo::Adaptador, pieza.as_bytes()).unwrap();
    let destino = concat!(
        "PawnCommon = {\n[\"Digits\"] = 1,\n[\"Scales\"] = {\n[\"S\"] = {\n[\"PerCharacterOptions\"] = {\n",
        "[\"Otro Pj\"] = {\n[\"Visible\"] = false,\n},\n},\n},\n},\n}\n"
    );
    let (contenido, elementos) = hecha(aplicar(
        &addon,
        pieza.as_bytes(),
        destino.as_bytes(),
        Some(PJ),
        None,
    ));
    assert!(contenido.contains("[\"Digits\"] = 2,"));
    assert!(!contenido.contains("Secreto"));
    assert!(contenido.contains("[\"Otro Pj\"] = {\n[\"Visible\"] = false,\n},"));
    assert!(contenido.contains("[\"Goldberg Joestar\"] = {\n[\"Visible\"] = true,\n},"));
    let nombres: Vec<_> = elementos.iter().map(|e| e.nombre.as_str()).collect();
    assert_eq!(nombres, ["PawnCommon.Digits", "PawnCommon.Scales"]);
    assert_eq!(
        aplicar(&addon, pieza.as_bytes(), destino.as_bytes(), None, None),
        Aplicacion::NoAplicable(MotivoNoAplicable::SinNombre)
    );
}

#[test]
fn generico_de_cuenta_cambia_sus_variables_y_deja_las_demas() {
    let e = entrada(
        "fichero-filtrado",
        "cuenta/SavedVariables/Dominos.lua",
        json!({ "claves_quitadas": ["DominosDB"] }),
    );
    let pieza = b"DominosVersion = 2\nDominosNueva = {\n}\n";
    let addon = clasificar(&e, Tipo::FicheroFiltrado, pieza).unwrap();
    let destino = "DominosDB = {\n[\"profiles\"] = {},\n}\nDominosVersion = 1\n";
    let (contenido, elementos) = hecha(aplicar(&addon, pieza, destino.as_bytes(), None, None));
    assert_eq!(
        contenido,
        "DominosDB = {\n[\"profiles\"] = {},\n}\nDominosVersion = 2\nDominosNueva = {\n}\n"
    );
    let cambios: Vec<_> = elementos
        .iter()
        .map(|e| (e.nombre.as_str(), e.cambio))
        .collect();
    assert_eq!(
        cambios,
        [
            ("DominosVersion", Cambio::Cambiado),
            ("DominosNueva", Cambio::Anadido)
        ]
    );
}

#[test]
fn generico_del_personaje_sustituye_el_fichero_y_cuenta_sus_variables() {
    let e = entrada("fichero", "personaje/SavedVariables/Dominos.lua", json!({}));
    let pieza = b"A = 2\nC = 1\nD = 4\n";
    let addon = clasificar(&e, Tipo::Fichero, pieza).unwrap();
    let (contenido, elementos) =
        hecha(aplicar(&addon, pieza, b"A = 1\nB = 3\nD = 4\n", None, None));
    assert_eq!(contenido.as_bytes(), pieza);
    let cambios: Vec<_> = elementos
        .iter()
        .map(|e| (e.nombre.as_str(), e.cambio))
        .collect();
    assert_eq!(
        cambios,
        [
            ("A", Cambio::Cambiado),
            ("C", Cambio::Anadido),
            ("B", Cambio::Quitado)
        ]
    );
}

#[test]
fn un_generico_de_cuenta_no_toca_ningun_acedb() {
    let e = entrada("fichero", "cuenta/SavedVariables/Dominos.lua", json!({}));
    let acedb = "DominosDB = {\n[\"profileKeys\"] = {\n[\"Otro Pj\"] = \"Mio\",\n},\n[\"profiles\"] = {\n},\n}\n";
    assert_eq!(
        clasificar(&e, Tipo::Fichero, acedb.as_bytes()).unwrap_err(),
        MotivoIncompatible::PiezaNoValida
    );
    let pieza = b"DominosDB = 1\nDominosVersion = 2\n";
    let addon = clasificar(&e, Tipo::Fichero, pieza).unwrap();
    let (contenido, elementos) = hecha(aplicar(&addon, pieza, acedb.as_bytes(), None, None));
    assert!(contenido.starts_with(acedb));
    assert!(contenido.contains("DominosVersion = 2"));
    let nombres: Vec<_> = elementos.iter().map(|e| e.nombre.as_str()).collect();
    assert_eq!(nombres, ["DominosVersion"]);
}

#[test]
fn lo_que_no_saldria_como_datos_no_se_escribe() {
    let e = entrada("fichero", "personaje/SavedVariables/Dominos.lua", json!({}));
    let addon = clasificar(&e, Tipo::Fichero, b"A = 1\n").unwrap();
    // `clasificar` ya rechaza esta pieza: aquí se prueba la última defensa, la que mira lo que se va a escribir.
    let codigo = b"A = RunScript(\"x\")\n";
    assert_eq!(
        aplicar(&addon, codigo, b"", None, None),
        Aplicacion::NoAplicable(MotivoNoAplicable::NoEncaja)
    );
    // Si nada cambia, no se escribe nada: un destino que ya no era solo datos no bloquea la entrada.
    let (contenido, _) = hecha(aplicar(&addon, codigo, codigo, None, None));
    assert_eq!(contenido.as_bytes(), codigo);
}

#[test]
fn una_variable_que_no_es_tabla_y_no_cambia_no_es_un_cambio() {
    let e = entrada(
        "adaptador",
        "personaje/SavedVariables/Auctionator.lua",
        json!({ "addon": "Auctionator", "variable": "AUCTIONATOR_CHARACTER_CONFIG", "adaptador": { "id": "auctionator", "version": 1 } }),
    );
    // Medido en una copia real (2026-11-18): la variable vale `nil` en origen y destino.
    let pieza = b"\nAUCTIONATOR_CHARACTER_CONFIG = nil\n";
    let addon = clasificar(&e, Tipo::Adaptador, pieza).unwrap();
    let (contenido, elementos) = hecha(aplicar(&addon, pieza, pieza, Some(PJ), None));
    assert_eq!(contenido.as_bytes(), pieza);
    assert!(elementos.is_empty());
    let (_, elementos) = hecha(aplicar(
        &addon,
        b"AUCTIONATOR_CHARACTER_CONFIG = {}\n",
        pieza,
        Some(PJ),
        None,
    ));
    assert_eq!(elementos.len(), 1);
}
