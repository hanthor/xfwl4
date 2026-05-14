#[cfg(not(feature = "custom_logger"))]
fn main() {}

#[cfg(feature = "custom_logger")]
fn main() {
    let library = pkg_config::probe_library("libseat").unwrap();

    let mut builder = cc::Build::new();
    builder
        .std("c11")
        .file("./log_handler/log_handler.c")
        .flag("-Wno-unused-parameter")
        .include("/usr/local/include");
    for i in &library.include_paths {
        builder.include(i);
    }
    builder.compile("libpreformatedlog");
}
