#[cfg(all(feature = "use_bindgen", not(any(docsrs, feature = "docs_rs"))))]
fn main() {
    use bindgen::Builder;

    let library = pkg_config::probe_library("libseat").unwrap();

    println!("cargo:rerun-if-changed=src/wrapper.h");

    let mut builder = Builder::default()
        .header("src/wrapper.h")
        .layout_tests(false)
        .allowlist_recursively(false)
        .allowlist_type("libseat.*")
        .allowlist_function("libseat.*")
        .allowlist_var("libseat.*")
        .blocklist_item("libseat_set_log_handler")
        .blocklist_item("libseat_log_func");
    for i in &library.include_paths {
        builder = builder.clang_arg(format!("-I{}", i.display()));
    }
    let bindings = builder.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file("./src/bindings.rs")
        .expect("Couldn't write bindings!");
}

#[cfg(all(not(feature = "use_bindgen"), not(any(docsrs, feature = "docs_rs"))))]
fn main() {
    pkg_config::probe_library("libseat").unwrap();
}

#[cfg(any(docsrs, feature = "docs_rs"))]
fn main() {}
