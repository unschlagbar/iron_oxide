fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(ui_ref_check)");

    // On in every build with debug assertions; the feature forces it on in release too.
    let debug = std::env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some();
    let forced = std::env::var_os("CARGO_FEATURE_UI_REF_CHECK").is_some();
    if debug || forced {
        println!("cargo::rustc-cfg=ui_ref_check");
    }
}
