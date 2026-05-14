use system_deps::Dependencies;

fn main() {
    if std::env::var("DOCS_RS").is_ok() {
        // don't link against unavailable native lib in doc.rs builds
        return;
    }

    let deps = system_deps::Config::new().probe().unwrap();
    auto_detect(&deps);
}

#[cfg(feature = "auto")]
fn auto_detect(deps: &Dependencies) {
    let native_lib = deps.get_by_name("libdisplay-info").unwrap();
    let native_version = semver::Version::parse(&native_lib.version).unwrap();
    let is_v3 = semver::VersionReq::parse(">=0.3")
        .unwrap()
        .matches(&native_version);
    if is_v3 {
        println!("cargo:rustc-cfg=feature=\"v0_3\"");
        return;
    }

    let is_v2 = semver::VersionReq::parse(">=0.2")
        .unwrap()
        .matches(&native_version);
    if is_v2 {
        println!("cargo:rustc-cfg=feature=\"v0_2\"");
        return;
    }

    println!("cargo:rustc-cfg=feature=\"v0_1\"");
}

#[cfg(not(feature = "auto"))]
fn auto_detect(_: &Dependencies) {}
