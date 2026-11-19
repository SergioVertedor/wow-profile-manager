use super::*;
use crate::deteccion::Producto;
use crate::toc::Toc;
use std::fs::{create_dir_all, write};

const PJ: &str = "Goldberg Joestar";

fn personaje(raiz: &Path, nombre: Option<&str>) -> PersonajeUbicado {
    let ubicado = PersonajeUbicado {
        raiz_version: raiz.join("_forever_"),
        producto: Producto::Forever,
        cuenta: "123#1".into(),
        reino: "93".into(),
        personaje: "Goldberg-Joestar".into(),
        nombre_en_datos: nombre.map(str::to_owned),
    };
    create_dir_all(ubicado.carpeta().join(CARPETA_SV)).unwrap();
    create_dir_all(ubicado.carpeta_cuenta().join(CARPETA_SV)).unwrap();
    ubicado
}

fn addon(carpeta: &str, version: Option<&str>) -> Addon {
    Addon {
        carpeta: carpeta.into(),
        toc: format!("{carpeta}.toc"),
        datos: Toc {
            version: version.map(str::to_owned),
            ..Toc::default()
        },
    }
}

fn en_cuenta(pj: &PersonajeUbicado, addon: &str, lua: &str) {
    write(
        pj.carpeta_cuenta()
            .join(CARPETA_SV)
            .join(format!("{addon}.lua")),
        lua,
    )
    .unwrap();
}

fn en_personaje(pj: &PersonajeUbicado, addon: &str, lua: &str) {
    write(
        pj.carpeta().join(CARPETA_SV).join(format!("{addon}.lua")),
        lua,
    )
    .unwrap();
}

fn config(pj: &PersonajeUbicado, addons: &[Addon]) -> ConfigAddons {
    de_personaje(
        pj,
        addons,
        &HashSet::from([PJ.to_owned(), "Otro Pj".to_owned()]),
    )
}

fn texto(pieza: &PiezaAddon) -> &str {
    std::str::from_utf8(&pieza.pieza.contenido).unwrap()
}

const DOMINOS: &str = concat!(
    "DominosDB = {\n",
    "[\"namespaces\"] = {\n",
    "[\"ProgressBars\"] = {\n",
    "[\"char\"] = {\n[\"Goldberg Joestar\"] = {\n[\"x\"] = 1,\n},\n},\n",
    "[\"profiles\"] = {\n[\"Druida\"] = {\n[\"bar\"] = 2,\n},\n[\"Otro\"] = {\n[\"bar\"] = 9,\n},\n},\n",
    "},\n",
    "[\"Vacio\"] = {\n},\n",
    "},\n",
    "[\"profileKeys\"] = {\n[\"Goldberg Joestar\"] = \"Druida\",\n[\"Otro Pj\"] = \"Otro\",\n},\n",
    "[\"global\"] = {\n[\"g\"] = 1,\n},\n",
    "[\"profiles\"] = {\n[\"Druida\"] = {\n[\"escala\"] = 1,\n},\n[\"Otro\"] = {\n[\"escala\"] = 3,\n},\n},\n",
    "}\n",
);

#[test]
fn perfil_acedb_lleva_su_perfil_y_el_de_cada_namespace_y_nada_mas() {
    let tmp = tempfile::tempdir().unwrap();
    let pj = personaje(tmp.path(), Some(PJ));
    en_cuenta(&pj, "Dominos", DOMINOS);

    let config = config(&pj, &[addon("Dominos", None)]);

    assert_eq!(config.piezas.len(), 1, "{:?}", config.no_exportables);
    let pieza = &config.piezas[0];
    assert_eq!(
        texto(pieza),
        concat!(
            "{\n[\"profile\"] = {\n[\"escala\"] = 1,\n},\n",
            "[\"namespaces\"] = {\n[\"ProgressBars\"] = {\n[\"bar\"] = 2,\n},\n},\n}\n"
        )
    );
    assert_eq!(pieza.pieza.tipo, Tipo::PerfilAcedb);
    assert_eq!(pieza.pieza.destino, "cuenta/SavedVariables/Dominos.lua");
    assert_eq!(pieza.pieza.id, "addon-dominos-perfil-dominosdb");
    assert_eq!(pieza.regla, Regla::PerfilAcedb);
    assert!(pieza.marcada);
    let de_addon = pieza.pieza.addon.as_ref().unwrap();
    assert_eq!(de_addon.perfil.as_deref(), Some("Druida"));
    assert_eq!(de_addon.variable.as_deref(), Some("DominosDB"));
    assert!(de_addon.marcadores.is_empty());
}

#[test]
fn un_perfil_con_el_nombre_del_personaje_viaja_con_el_marcador() {
    let tmp = tempfile::tempdir().unwrap();
    let pj = personaje(tmp.path(), Some(PJ));
    en_cuenta(
        &pj,
        "Spyish",
        "SpyishDB = {\n[\"profileKeys\"] = {\n[\"Goldberg Joestar\"] = \"Goldberg Joestar\",\n},\n[\"profiles\"] = {\n[\"Goldberg Joestar\"] = {\n},\n},\n}\n",
    );

    let config = config(&pj, &[addon("Spyish", None)]);

    let de_addon = config.piezas[0].pieza.addon.as_ref().unwrap();
    assert_eq!(de_addon.perfil.as_deref(), Some(MARCADOR_PERSONAJE));
    assert_eq!(de_addon.marcadores, [Marcador::Personaje]);
    assert_eq!(config.piezas[0].perfil.as_deref(), Some(PJ));
}

#[test]
fn sin_perfil_asignado_o_sin_nombre_no_se_exporta_y_se_dice() {
    let tmp = tempfile::tempdir().unwrap();
    let pj = personaje(tmp.path(), Some("Goldberg Sp"));
    en_cuenta(&pj, "Dominos", DOMINOS);
    let sin_perfil = config(&pj, &[addon("Dominos", None)]);
    assert!(sin_perfil.piezas.is_empty());
    assert_eq!(
        sin_perfil.no_exportables[0].motivo,
        MotivoNoExportable::SinPerfil
    );
    assert_eq!(
        sin_perfil.no_exportables[0].variable.as_deref(),
        Some("DominosDB")
    );

    let sin_nombre = config(&personaje(tmp.path(), None), &[addon("Dominos", None)]);
    assert_eq!(
        sin_nombre.no_exportables[0].motivo,
        MotivoNoExportable::SinNombre
    );
}

#[test]
fn el_resto_de_un_fichero_con_acedb_sale_desmarcado_sin_las_variables_acedb() {
    let tmp = tempfile::tempdir().unwrap();
    let pj = personaje(tmp.path(), Some(PJ));
    let lua = format!("{DOMINOS}DominosVersion = 3\nDominosOtra = {{\n[\"k\"] = 1,\n}}\n");
    en_cuenta(&pj, "Dominos", &lua);

    let config = config(&pj, &[addon("Dominos", None)]);

    let resto = &config.piezas[1];
    assert_eq!(
        texto(resto),
        "DominosVersion = 3\nDominosOtra = {\n[\"k\"] = 1,\n}\n"
    );
    assert_eq!(resto.pieza.tipo, Tipo::FicheroFiltrado);
    assert_eq!(resto.regla, Regla::GenericoCuenta);
    assert!(!resto.marcada);
    assert_eq!(
        resto.pieza.quitadas,
        [Quitadas {
            motivo: Motivo::ViajaComoPerfil,
            claves: vec!["DominosDB".into()]
        }]
    );
}

#[test]
fn genericos_enteros_desmarcados_con_su_regla_y_pistas() {
    let tmp = tempfile::tempdir().unwrap();
    let pj = personaje(tmp.path(), Some(PJ));
    en_cuenta(&pj, "Limpio", "LimpioDB = {\n[\"escala\"] = 1,\n}\n");
    en_cuenta(
        &pj,
        "Menciona",
        "MencionaDB = {\n[\"Otro Pj\"] = {\n},\n}\n",
    );
    let con_pistas =
        "PorPjDB = {\n[\"amigo\"] = \"Player-4-0A1B2C3D\",\n[\"Thrall - Durotar\"] = 1,\n}\n";
    en_personaje(&pj, "PorPj", con_pistas);

    let config = config(
        &pj,
        &[
            addon("Limpio", None),
            addon("Menciona", None),
            addon("PorPj", None),
        ],
    );

    let reglas: Vec<_> = config.piezas.iter().map(|p| (p.regla, p.marcada)).collect();
    assert_eq!(
        reglas,
        [
            (Regla::GenericoCuenta, false),
            (Regla::GenericoCuentaConPersonajes, false),
            (Regla::GenericoPersonaje, false),
        ]
    );
    assert_eq!(config.piezas[0].pieza.tipo, Tipo::Fichero);
    assert!(config.piezas[0].pistas.is_empty());
    assert_eq!(texto(&config.piezas[2]), con_pistas);
    assert_eq!(config.piezas[2].pistas, [Pista::Guid, Pista::NombreReino]);
    assert_eq!(
        config.piezas[2].pieza.destino,
        "personaje/SavedVariables/PorPj.lua"
    );
}

#[test]
fn adaptador_solo_lleva_su_lista_blanca_y_cambia_el_personaje_por_el_marcador() {
    let tmp = tempfile::tempdir().unwrap();
    let pj = personaje(tmp.path(), Some(PJ));
    en_cuenta(
        &pj,
        "Pawn",
        concat!(
            "PawnCommon = {\n[\"Digits\"] = 1,\n[\"LastVersion\"] = 2,\n[\"Scales\"] = {\n[\"S\"] = {\n",
            "[\"PerCharacterOptions\"] = {\n[\"Goldberg Joestar\"] = {\n[\"Visible\"] = true,\n},\n",
            "[\"Otro Pj\"] = {\n[\"Visible\"] = false,\n},\n},\n},\n},\n}\nPawnOtra = 1\n"
        ),
    );
    en_personaje(
        &pj,
        "Pawn",
        "PawnOptions = {\n[\"AutoSelectScales\"] = true,\n[\"ItemLevels\"] = {\n},\n}\n",
    );

    let config = config(&pj, &[addon("Pawn", Some("9.9"))]);

    assert_eq!(config.piezas.len(), 2);
    let cuenta = &config.piezas[0];
    assert_eq!(cuenta.pieza.tipo, Tipo::Adaptador);
    assert!(cuenta.marcada && !cuenta.version_no_medida);
    assert!(texto(cuenta).starts_with("PawnCommon = {\n[\"Digits\"] = 1,\n[\"Scales\"]"));
    assert!(texto(cuenta).contains("[\"@@WOWPROFILE:PERSONAJE@@\"] = {\n[\"Visible\"] = true,\n}"));
    assert!(!texto(cuenta).contains("Otro Pj") && !texto(cuenta).contains("LastVersion"));
    let de_addon = cuenta.pieza.addon.as_ref().unwrap();
    assert_eq!(de_addon.marcadores, [Marcador::Personaje]);
    assert_eq!(
        de_addon.adaptador,
        Some(RefAdaptador {
            id: "pawn".into(),
            version: 2
        })
    );
    assert_eq!(
        texto(&config.piezas[1]),
        "PawnOptions = {\n[\"AutoSelectScales\"] = true,\n}\n"
    );
}

#[test]
fn adaptador_con_otra_version_sale_desmarcado_y_no_exportar_no_aparece() {
    let tmp = tempfile::tempdir().unwrap();
    let pj = personaje(tmp.path(), Some(PJ));
    en_cuenta(
        &pj,
        "WIM",
        "WIM3_Data = {\n[\"a\"] = 1,\n}\nWIM3_History = {\n[\"x\"] = 1,\n}\n",
    );
    en_cuenta(&pj, "LayerSwap", "LayerSwapDB = {\n}\n");

    let config = config(&pj, &[addon("WIM", Some("4.0")), addon("LayerSwap", None)]);

    assert_eq!(config.piezas.len(), 1);
    assert_eq!(texto(&config.piezas[0]), "WIM3_Data = {\n[\"a\"] = 1,\n}\n");
    assert!(!config.piezas[0].marcada && config.piezas[0].version_no_medida);
    assert!(config.no_exportables.is_empty());
}

#[test]
fn lo_que_no_parsea_o_trae_el_marcador_se_dice_y_lo_desinstalado_no_aparece() {
    let tmp = tempfile::tempdir().unwrap();
    let pj = personaje(tmp.path(), Some(PJ));
    en_cuenta(&pj, "Roto", "RotoDB = {\n[\"a\"] = \n");
    en_cuenta(&pj, "Marcado", "MarcadoDB = \"@@WOWPROFILE:PERSONAJE@@\"\n");
    en_cuenta(&pj, "Desinstalado", "DesinstaladoDB = 1\n");
    en_personaje(&pj, "Blizzard_RaidUI", "RaidUIDB = 1\n");

    let config = config(&pj, &[addon("Roto", None), addon("Marcado", None)]);

    assert!(config.piezas.is_empty());
    let motivos: Vec<_> = config.no_exportables.iter().map(|n| n.motivo).collect();
    assert_eq!(
        motivos,
        [
            MotivoNoExportable::NoSeLee,
            MotivoNoExportable::TraeMarcador
        ]
    );
}

#[test]
fn ids_en_el_alfabeto_del_formato_cortos_y_unicos() {
    let mut usados = HashSet::new();
    assert_eq!(
        id_unico(&mut usados, PREFIJO_ID, "Prat-3.0", "cuenta"),
        "addon-prat-3-0-cuenta"
    );
    assert_eq!(
        id_unico(&mut usados, PREFIJO_ID, "prat_3.0", "cuenta"),
        "addon-prat-3-0-cuenta-2"
    );
    assert_eq!(
        id_unico(&mut usados, PREFIJO_ID, "!BugGrabber", "perfil-X"),
        "addon-buggrabber-perfil-x"
    );
    let largo = id_unico(&mut usados, PREFIJO_ID, &"A".repeat(80), "cuenta");
    assert!(largo.len() <= MAX_ID && !largo.ends_with('-'));
    let otro = id_unico(&mut usados, PREFIJO_ID, &"A".repeat(80), "cuenta");
    assert!(otro.len() <= MAX_ID && otro != largo);
}

#[test]
fn pistas_de_nombre_reino_y_guid() {
    assert!(es_nombre_reino("Thrall-Durotar"));
    assert!(es_nombre_reino("Goldberg - C'Thun"));
    assert!(es_nombre_reino("Gøldberg - Dun Modr"));
    assert!(!es_nombre_reino("Goldberg Joestar"));
    assert!(!es_nombre_reino("prat-module"));
    assert!(!es_nombre_reino("X-Y"));
    assert!(tiene_guid(b"\"Player-4-0A1B2C3D\""));
    assert!(!tiene_guid(b"Player-x-0A1B2C3D"));
    assert!(!tiene_guid(b"Player-4-0A1B"));
}

#[test]
fn una_carpeta_que_no_cabe_en_el_destino_se_dice_y_no_sale() {
    let tmp = tempfile::tempdir().unwrap();
    let pj = personaje(tmp.path(), Some(PJ));
    en_cuenta(&pj, "Raro..Addon", "RaroDB = 1\n");
    en_cuenta(&pj, "Bien.Addon", "BienDB = 1\n");

    let config = config(
        &pj,
        &[addon("Raro..Addon", None), addon("Bien.Addon", None)],
    );

    assert_eq!(config.piezas.len(), 1);
    assert_eq!(
        config.piezas[0].pieza.destino,
        "cuenta/SavedVariables/Bien.Addon.lua"
    );
    assert_eq!(
        config.no_exportables[0].motivo,
        MotivoNoExportable::CarpetaNoAdmitida
    );
}
