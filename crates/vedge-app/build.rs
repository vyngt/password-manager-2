use leptos_i18n_build::options::CodegenOptions;
use leptos_i18n_build::{Config, FileFormat, ParseOptions, TranslationsInfos};
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=Cargo.toml");

    // Root is okay
    let i18n_mod_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let options = ParseOptions::default().file_format(FileFormat::Yaml);

    let cfg = Config::new("en")?
        .add_locale("vi")?
        .add_namespaces([
            "unlock",
            "vault",
            "window",
            "playground",
            "date_picker",
            "onboarding",
            "nav",
            "settings",
            "generator",
            "audit",
        ])?
        .parse_options(options);

    let translations_infos = TranslationsInfos::parse(cfg)?;
    translations_infos.emit_diagnostics();
    translations_infos.rerun_if_locales_changed();

    let mut codegen_options = CodegenOptions::default();
    let filename = PathBuf::from("i18n.rs");
    codegen_options.module_file_name = &filename;

    translations_infos.generate_i18n_module_with_options(i18n_mod_directory, codegen_options)?;

    Ok(())
}
