use super::*;
use std::fs::{create_dir_all, write};

/// Un ZIP hecho a mano, con los nombres tal cual (aunque sean peligrosos).
fn zip_con(entradas: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let opciones = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (nombre, contenido) in entradas {
        zip.start_file(*nombre, opciones).unwrap();
        zip.write_all(contenido).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn rutas(ficheros: &[FicheroAddon]) -> Vec<&str> {
    ficheros.iter().map(|f| f.ruta.as_str()).collect()
}

#[test]
fn una_carpeta_va_y_vuelve_con_sus_subcarpetas_y_en_orden() {
    let tmp = tempfile::tempdir().unwrap();
    let addon = tmp.path().join("MiAddon");
    create_dir_all(addon.join("Media/Iconos")).unwrap();
    write(addon.join("MiAddon.toc"), "## Title: Mi addon").unwrap();
    write(addon.join("nucleo.lua"), "local x = 1").unwrap();
    write(addon.join("Media/Iconos/a.tga"), [0u8, 1, 2, 255]).unwrap();

    let ficheros = leer(&empaquetar(&addon).unwrap()).unwrap();

    assert_eq!(
        rutas(&ficheros),
        ["Media/Iconos/a.tga", "MiAddon.toc", "nucleo.lua"]
    );
    assert_eq!(ficheros[0].contenido, [0u8, 1, 2, 255]);
    assert_eq!(ficheros[2].contenido, b"local x = 1");
}

#[test]
fn ninguna_ruta_sale_de_la_carpeta_del_addon() {
    for malo in [
        "../fuera.lua",
        "a/../../fuera.lua",
        "/absoluta.lua",
        "C:/Windows/x.lua",
        "a\\b.lua",
        "./a.lua",
        "a//b.lua",
        "CON.lua",
        "nombre. ",
        "flujo:alterno",
    ] {
        assert_eq!(
            leer(&zip_con(&[(malo, b"x")])),
            Err(ErrorAddon::RutaNoValida(malo.to_owned())),
            "{malo}"
        );
    }
}

#[test]
fn dos_ficheros_que_en_windows_son_el_mismo_no_se_aceptan() {
    let zip = zip_con(&[("A.lua", b"1"), ("a.lua", b"2")]);

    assert_eq!(leer(&zip), Err(ErrorAddon::RutaNoValida("a.lua".into())));
}

#[test]
fn un_enlace_dentro_del_zip_no_se_acepta() {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.add_symlink("enlace", "../../fuera", SimpleFileOptions::default())
        .unwrap();
    let bytes = zip.finish().unwrap().into_inner();

    assert_eq!(leer(&bytes), Err(ErrorAddon::Enlace("enlace".into())));
}

#[test]
fn descomprimir_para_en_el_limite_aunque_el_zip_ocupe_poco() {
    // Una «bomba»: muchos ceros comprimen casi a nada.
    let ceros = vec![0u8; usize::try_from(MAX_BYTES_ADDON).unwrap() + 1];
    let zip = zip_con(&[("grande.bin", &ceros)]);
    assert!(zip.len() < 1024 * 1024);

    assert_eq!(leer(&zip), Err(ErrorAddon::DemasiadoGrande));
}

#[test]
fn demasiados_ficheros_no_se_aceptan() {
    let nombres: Vec<String> = (0..=MAX_FICHEROS_ADDON)
        .map(|i| format!("f{i}.lua"))
        .collect();
    let entradas: Vec<(&str, &[u8])> = nombres.iter().map(|n| (n.as_str(), &b""[..])).collect();

    assert_eq!(
        leer(&zip_con(&entradas)),
        Err(ErrorAddon::DemasiadosFicheros)
    );
}

#[test]
fn una_carpeta_demasiado_grande_no_se_empaqueta() {
    let tmp = tempfile::tempdir().unwrap();
    let addon = tmp.path().join("Grande");
    create_dir_all(&addon).unwrap();
    let mitad = usize::try_from(MAX_BYTES_ADDON / 2).unwrap() + 1;
    write(addon.join("a.bin"), vec![0u8; mitad]).unwrap();
    write(addon.join("b.bin"), vec![0u8; mitad]).unwrap();

    assert_eq!(empaquetar(&addon), Err(ErrorAddon::DemasiadoGrande));
}

#[test]
fn lo_que_no_es_un_zip_no_se_lee() {
    assert!(matches!(
        leer(b"no soy un zip"),
        Err(ErrorAddon::NoSeLee(_))
    ));
}

#[test]
fn medir_cuenta_lo_que_se_empaquetaria_sin_leerlo() {
    let tmp = tempfile::tempdir().unwrap();
    let addon = tmp.path().join("MiAddon");
    create_dir_all(addon.join("Media")).unwrap();
    write(addon.join("MiAddon.toc"), "12345").unwrap();
    write(addon.join("Media/a.tga"), [0u8; 10]).unwrap();

    assert_eq!(
        medir(&addon),
        Ok(Medida {
            bytes: 15,
            ficheros: 2
        })
    );
    write(addon.join("CON.lua"), "x").unwrap();
    assert_eq!(
        medir(&addon),
        Err(ErrorAddon::RutaNoValida("CON.lua".to_owned()))
    );
}

/// Una unión de Windows (`mklink /J`, sin permisos de administrador) dentro del addon no se sigue: sacaría ficheros
/// de fuera de su carpeta.
#[cfg(windows)]
#[test]
fn una_union_dentro_del_addon_no_se_sigue() {
    let tmp = tempfile::tempdir().unwrap();
    let addon = tmp.path().join("MiAddon");
    let fuera = tmp.path().join("Fuera");
    create_dir_all(&addon).unwrap();
    create_dir_all(&fuera).unwrap();
    write(fuera.join("secreto.txt"), "no viaja").unwrap();
    let creada = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(addon.join("Libs"))
        .arg(&fuera)
        .output()
        .unwrap();
    assert!(creada.status.success(), "{creada:?}");

    assert_eq!(medir(&addon), Err(ErrorAddon::Enlace("Libs".to_owned())));
    assert_eq!(
        empaquetar(&addon),
        Err(ErrorAddon::Enlace("Libs".to_owned()))
    );
}

#[test]
fn una_carpeta_del_zip_tampoco_puede_salirse() {
    for mala in ["../", "a/../../", "CON/"] {
        assert_eq!(
            leer(&zip_con(&[(mala, b"")])),
            Err(ErrorAddon::RutaNoValida(mala.to_owned())),
            "{mala}"
        );
    }
    assert!(leer(&zip_con(&[("Media/", b""), ("Media/a.tga", b"x")])).is_ok());
}
