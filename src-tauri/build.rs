fn main() {
    // Tauri incrusta en el ejecutable de la app el manifiesto que pide Common Controls v6; los binarios de test no
    // lo llevan y en Windows no arrancan (STATUS_ENTRYPOINT_NOT_FOUND). Se lo damos solo a los tests.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifiesto =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifiesto.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifiesto.display()
        );
    }
    tauri_build::build()
}
