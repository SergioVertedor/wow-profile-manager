use super::*;

/// Clave y firma **de prueba**, generadas con `tauri signer` solo para estos tests (no es la clave de la app).
const CLAVE_PRUEBA: &str = "RWTvIXeEKkNAeIdLxPcvy3YPAr6JntUtyJk73K+cynILc0/DzPIGBw9e";
const EXE_PRUEBA: &[u8] = b"exe de prueba";
const SIG_PRUEBA: &[u8] = b"dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVUdklYZUVLa05BZUhuNUNXL3l3WFliQ0c1QndINlg2Rk94bkJxRGxHRS91UUdKTHI3OGxNZXVxeklTdHl4U2NLczIvOElHR0kzSVRtZWN3YVRvcU5naFpZZ2gvR2tGdVE0PQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk1MDkyMDEwCWZpbGU6YXBwLmV4ZQpiMkpCR2VCc0IvQUtMa2lMZVdObmlON3RIalZ1RmxFbWMrcURpL1AvTStmbk41VHpON1djWFlQSmtjOWtSWlJnaStQMnNBcXJlUjd3eFMyNWhjWXlBZz09Cg==";

fn release(tag: &str, ficheros: &[&str]) -> Release {
    Release {
        tag_name: tag.to_owned(),
        body: Some("Novedades".to_owned()),
        assets: ficheros
            .iter()
            .map(|nombre| Fichero {
                name: (*nombre).to_owned(),
                browser_download_url: format!("https://ejemplo/{nombre}"),
            })
            .collect(),
    }
}

#[test]
fn las_versiones_se_comparan_por_numeros() {
    let v = |t| Version::leer(t).unwrap();

    assert!(v("0.10.0") > v("0.9.9"));
    assert!(v("v1.0.0") > v("0.99.99"));
    assert_eq!(v("v0.2.1"), v("0.2.1"));
    assert_eq!(v("1.2.3").to_string(), "1.2.3");
    for raro in ["1.2", "1.2.3.4", "1.2.3-beta", "x.1.2", "", "v"] {
        assert_eq!(Version::leer(raro), None, "{raro}");
    }
}

#[test]
fn solo_se_ofrece_una_version_mas_nueva_con_sus_dos_ficheros() {
    let actual = Version::leer("0.1.0").unwrap();
    let completos = [NOMBRE_EXE, NOMBRE_FIRMA];

    let nueva = disponible(&release("v0.2.0", &completos), actual).unwrap();
    assert_eq!(nueva.version, "0.2.0");
    assert_eq!(nueva.notas, "Novedades");
    assert_eq!(nueva.url_exe, format!("https://ejemplo/{NOMBRE_EXE}"));
    assert_eq!(nueva.url_firma, format!("https://ejemplo/{NOMBRE_FIRMA}"));

    assert_eq!(disponible(&release("v0.1.0", &completos), actual), None);
    assert_eq!(disponible(&release("v0.0.9", &completos), actual), None);
    assert_eq!(disponible(&release("v0.2.0", &[NOMBRE_EXE]), actual), None);
    assert_eq!(
        disponible(&release("v0.2.0-beta", &completos), actual),
        None
    );
}

#[test]
fn las_urls_no_salen_hacia_la_interfaz() {
    let actual = Version::leer("0.1.0").unwrap();
    let nueva = disponible(&release("v0.2.0", &[NOMBRE_EXE, NOMBRE_FIRMA]), actual).unwrap();

    let json = serde_json::to_value(nueva).unwrap();

    assert_eq!(
        json,
        serde_json::json!({ "version": "0.2.0", "notas": "Novedades" })
    );
}

#[test]
fn lee_el_json_de_la_api_de_github() {
    let json = r#"{"tag_name":"v0.2.0","body":null,"draft":false,
        "assets":[{"name":"wow-profile-manager.exe","browser_download_url":"https://x/a","size":1}]}"#;

    let release: Release = serde_json::from_str(json).unwrap();

    assert_eq!(release.tag_name, "v0.2.0");
    assert_eq!(release.body, None);
    assert_eq!(release.assets[0].name, NOMBRE_EXE);
}

#[test]
fn una_firma_buena_pasa() {
    assert_eq!(verificar(EXE_PRUEBA, SIG_PRUEBA, CLAVE_PRUEBA), Ok(()));
}

#[test]
fn un_exe_cambiado_o_firmado_con_otra_clave_no_pasa() {
    assert_eq!(
        verificar(b"exe de prueba cambiado", SIG_PRUEBA, CLAVE_PRUEBA),
        Err(FirmaNoValida::NoCuadra)
    );
    // La clave de la app no firmó el exe de prueba.
    assert_eq!(
        verificar(EXE_PRUEBA, SIG_PRUEBA, CLAVE_PUBLICA),
        Err(FirmaNoValida::NoCuadra)
    );
}

#[test]
fn una_firma_que_no_es_una_firma_no_pasa() {
    for sig in [&b""[..], b"no es base64 !!", b"aG9sYQ=="] {
        assert_eq!(
            verificar(EXE_PRUEBA, sig, CLAVE_PRUEBA),
            Err(FirmaNoValida::Formato)
        );
    }
}

#[test]
fn la_clave_de_la_app_es_una_clave_minisign_valida() {
    assert!(PublicKey::from_base64(CLAVE_PUBLICA).is_ok());
}

/// Sale a la red: solo a mano (`cargo test -- --ignored`). Sin release publicada, GitHub responde 404.
#[test]
#[ignore = "sale a la red"]
fn consulta_la_api_de_github_de_verdad() {
    match buscar(Version::MINIMA) {
        Ok(disponible) => println!("disponible: {disponible:?}"),
        Err(error) => println!("error: {error:?}"),
    }
}
