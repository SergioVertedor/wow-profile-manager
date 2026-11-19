use super::*;

/// Cabecera con la forma medida: versión, cuántos números siguen y esos números.
const CABECERA: &str = "5 3 0 100 1";
/// Dos marcos con la forma medida. La posición del segundo sale de `semilla`: cada diseño de prueba es distinto.
fn marcos(semilla: &str) -> String {
    let x: u32 = semilla.bytes().map(u32::from).sum();
    format!("2 0 0 1 7 7 UIParent 0.0 -4.0 -1 ##$$ 6 0 0 4 4 UIParent {x}.0 -275.5 -1 ##$#%#&.(()(")
}

fn diseno(nombre: &str) -> String {
    diseno_con(nombre, nombre)
}

/// Un diseño llamado `nombre` con los marcos del diseño de prueba `de`.
fn diseno_con(nombre: &str, de: &str) -> String {
    format!("{} {nombre} 0 {}", nombre.len(), marcos(de))
}

fn con_cabecera(disenos: &[String]) -> String {
    format!("{CABECERA} {}\0", disenos.join(" "))
}

fn cuenta(nombres: &[&str]) -> String {
    let disenos: Vec<String> = nombres.iter().map(|n| diseno(n)).collect();
    con_cabecera(&disenos)
}

#[test]
fn lee_la_cabecera_y_cada_diseno_con_sus_bytes() {
    let texto = cuenta(&["default", "Prueba1"]);

    let leida = Cuenta::leer(&texto).unwrap();

    assert_eq!(leida.cabecera(), CABECERA);
    let nombres: Vec<&str> = leida.disenos.iter().map(|d| d.nombre.as_str()).collect();
    assert_eq!(nombres, ["default", "Prueba1"]);
    assert_eq!(leida.texto_de(&leida.disenos[1]), diseno("Prueba1"));
}

#[test]
fn un_nombre_con_espacios_se_corta_por_su_largo() {
    let texto = cuenta(&["Mi diseño de banda"]);

    let leida = Cuenta::leer(&texto).unwrap();

    assert_eq!(leida.disenos[0].nombre, "Mi diseño de banda");
}

#[test]
fn una_cuenta_sin_disenos_propios_se_lee() {
    let texto = format!("{CABECERA}\0");

    assert!(Cuenta::leer(&texto).unwrap().disenos.is_empty());
}

#[test]
fn lo_que_no_cuadra_no_se_entiende() {
    let otra_version = cuenta(&["default"]).replacen('5', "6", 1);
    let cortado = cuenta(&["default"])
        .trim_end()
        .rsplit_once(' ')
        .unwrap()
        .0
        .to_owned();

    assert_eq!(
        Cuenta::leer(&otra_version).unwrap_err(),
        FormatoNoValido::Version
    );
    assert_eq!(
        Cuenta::leer(&cortado).unwrap_err(),
        FormatoNoValido::Diseno { posicion: 0 }
    );
    assert_eq!(Cuenta::leer("").unwrap_err(), FormatoNoValido::Cabecera);
    assert_eq!(
        Cuenta::leer("5 9 1 2").unwrap_err(),
        FormatoNoValido::Cabecera
    );
}

#[test]
fn el_personaje_usa_el_diseno_de_su_indice() {
    let texto = cuenta(&["default", "Prueba1"]);
    let leida = Cuenta::leer(&texto).unwrap();

    let usa = |personaje: &str| diseno_activo(&leida, personaje).unwrap().map(|d| d.nombre);

    assert_eq!(
        usa("5 3 3 3 3 3 -1 -1 -1 -1 -1\0").as_deref(),
        Some("default")
    );
    assert_eq!(
        usa("5 4 3 3 3 3 -1 -1 -1 -1 -1\0").as_deref(),
        Some("Prueba1")
    );
    // 0–2: los de Blizzard, que no viajan.
    assert_eq!(usa("5 0 3 3 3 3 -1 -1 -1 -1 -1\0"), None);
    assert_eq!(usa("5 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1\0"), None);
}

#[test]
fn un_indice_sin_diseno_o_un_personaje_raro_es_un_error() {
    let texto = cuenta(&["default"]);
    let leida = Cuenta::leer(&texto).unwrap();

    assert_eq!(
        diseno_activo(&leida, "5 4 3 3 3 3 -1 -1 -1 -1 -1").unwrap_err(),
        FormatoNoValido::IndiceFueraDeRango {
            indice: 4,
            disenos: 1
        }
    );
    assert_eq!(
        indice_activo("5 3 3").unwrap_err(),
        FormatoNoValido::Personaje
    );
    assert_eq!(indice_activo("x").unwrap_err(), FormatoNoValido::Personaje);
}

#[test]
fn el_paquete_es_un_fichero_de_cuenta_con_un_solo_diseno() {
    let texto = cuenta(&["default", "Prueba1"]);
    let leida = Cuenta::leer(&texto).unwrap();

    let paquete = paquete_de(&leida, &leida.disenos[1]);

    assert_eq!(paquete, cuenta(&["Prueba1"]));
}

#[test]
fn anadir_deja_intacto_lo_que_habia_y_da_el_indice_nuevo() {
    let destino = cuenta(&["default", "Otro"]);

    let anadido = anadir(Some(&destino), &cuenta(&["Sanador"])).unwrap();

    // Todo lo que había (sin el byte nulo final) sigue igual, byte a byte.
    assert!(anadido.texto.starts_with(destino.trim_end_matches('\0')));
    assert_eq!(anadido.texto, cuenta(&["default", "Otro", "Sanador"]));
    assert_eq!(anadido.nombre, "Sanador");
    assert_eq!(anadido.indice, 5);
}

#[test]
fn un_nombre_repetido_se_numera_con_su_largo_correcto() {
    let destino = cuenta(&["default", "default 2"]);
    // Mismo nombre, otros marcos: es otro diseño.
    let paquete = con_cabecera(&[diseno_con("default", "otro")]);

    let anadido = anadir(Some(&destino), &paquete).unwrap();

    assert_eq!(anadido.nombre, "default 3");
    assert!(
        anadido
            .texto
            .ends_with(&format!(" {}\0", diseno_con("default 3", "otro")))
    );
    assert_eq!(Cuenta::leer(&anadido.texto).unwrap().disenos.len(), 3);
}

#[test]
fn sin_fichero_de_cuenta_se_crea_con_la_cabecera_del_paquete() {
    let anadido = anadir(None, &cuenta(&["Sanador"])).unwrap();

    assert_eq!(anadido.texto, cuenta(&["Sanador"]));
    assert_eq!(anadido.indice, PRIMER_PROPIO);
}

#[test]
fn un_destino_que_no_se_entiende_no_se_toca() {
    assert_eq!(
        anadir(Some("6 0\0"), &cuenta(&["Sanador"])).unwrap_err(),
        FormatoNoValido::Version
    );
    assert_eq!(
        anadir(Some(&cuenta(&["a"])), &cuenta(&["b", "c"])).unwrap_err(),
        FormatoNoValido::Diseno { posicion: 0 }
    );
}

#[test]
fn el_personaje_pasa_a_usar_el_diseno_sin_tocar_el_resto() {
    assert_eq!(
        con_indice(Some("5 3 3 3 3 3 -1 -1 -1 -1 -1\0"), 5).unwrap(),
        "5 5 3 3 3 3 -1 -1 -1 -1 -1\0"
    );
    assert_eq!(con_indice(None, 4).unwrap(), "5 4 4 4 4 4 -1 -1 -1 -1 -1\0");
    assert_eq!(
        con_indice(Some("5 3"), 4).unwrap_err(),
        FormatoNoValido::Personaje
    );
}

#[test]
fn el_largo_del_nombre_va_en_bytes_como_lo_escribe_el_juego() {
    // Medido: el juego guarda `Diseño ñ` como `10 Diseño ñ`.
    let anadido = anadir(Some(&cuenta(&["default"])), &cuenta(&["Diseño ñ"])).unwrap();

    assert!(anadido.texto.contains(" 10 Diseño ñ 0 "));
    assert_eq!(
        Cuenta::leer(&anadido.texto).unwrap().disenos[1].nombre,
        "Diseño ñ"
    );
}

#[test]
fn sin_byte_nulo_final_tambien_se_lee_y_al_escribir_se_pone() {
    let sin_nulo = cuenta(&["default"]).trim_end_matches('\0').to_owned();

    let anadido = anadir(Some(&sin_nulo), &cuenta(&["Sanador"])).unwrap();

    assert_eq!(anadido.texto, cuenta(&["default", "Sanador"]));
}

#[test]
fn un_diseno_identico_ya_en_el_destino_se_reutiliza_sin_anadir_nada() {
    let destino = cuenta(&["default", "Sanador"]);

    let paquete = con_cabecera(&[diseno_con("Otro nombre", "Sanador")]);

    let anadido = anadir(Some(&destino), &paquete).unwrap();

    assert_eq!(anadido.texto, destino);
    assert_eq!(
        (anadido.nombre.as_str(), anadido.indice, anadido.nuevo),
        ("Sanador", 4, false)
    );
}

#[test]
fn exportar_e_importar_llevan_el_diseno_activo_de_punta_a_punta() {
    let origen = cuenta(&["default", "Sanador"]);
    let (nombre, paquete) = exportar(origen.as_bytes(), b"5 4 3 3 3 3 -1 -1 -1 -1 -1\0")
        .unwrap()
        .unwrap();
    assert_eq!(nombre, "Sanador");

    let importado = importar(Some(cuenta(&["default"]).as_bytes()), None, &paquete).unwrap();

    assert!(importado.nuevo);
    assert_eq!(importado.nombre, "Sanador");
    assert_eq!(importado.personaje, b"5 4 4 4 4 4 -1 -1 -1 -1 -1\0");
    let texto = String::from_utf8(importado.cuenta).unwrap();
    assert_eq!(Cuenta::leer(&texto).unwrap().disenos.len(), 2);
}

#[test]
fn con_un_diseno_de_blizzard_no_hay_nada_que_exportar() {
    let origen = cuenta(&["default"]);

    assert_eq!(
        exportar(origen.as_bytes(), b"5 1 3 3 3 3 -1 -1 -1 -1 -1\0").unwrap(),
        None
    );
    assert_eq!(
        exportar(&[0xff, 0xfe], b"5 3").unwrap_err(),
        FormatoNoValido::NoEsTexto
    );
}

#[test]
fn reutilizar_un_diseno_devuelve_el_destino_byte_a_byte() {
    // Aunque el destino no acabe como el juego lo escribe, si no se añade nada no se toca.
    let destino = format!("{} ", cuenta(&["Sanador"]).trim_end_matches('\0'));

    let anadido = anadir(Some(&destino), &cuenta(&["Sanador"])).unwrap();

    assert!(!anadido.nuevo);
    assert_eq!(anadido.texto, destino);
}

#[test]
fn un_nombre_mal_cortado_raro_o_con_un_largo_enorme_no_es_un_diseno() {
    let malo = |diseno: &str| Cuenta::leer(&format!("{CABECERA} {diseno}\0")).unwrap_err();
    let error = FormatoNoValido::Diseno { posicion: 0 };

    // Largo que no acaba en el espacio tras el nombre.
    assert_eq!(malo(&format!("3 Sanador 0 {}", marcos("x"))), error);
    // Vacío, con caracteres de control o con un largo que desbordaría.
    assert_eq!(malo(&format!("0  0 {}", marcos("x"))), error);
    assert_eq!(malo(&format!("2 a\n 0 {}", marcos("x"))), error);
    assert_eq!(malo(&format!("{} a 0 {}", usize::MAX, marcos("x"))), error);
    // El tipo tiene que ser un número.
    assert_eq!(malo(&format!("1 a x {}", marcos("x"))), error);
}

#[test]
fn un_personaje_con_un_numero_de_ranuras_enorme_no_se_entiende() {
    assert_eq!(
        indice_activo(&format!("{} 3 3\0", usize::MAX)).unwrap_err(),
        FormatoNoValido::Personaje
    );
}
