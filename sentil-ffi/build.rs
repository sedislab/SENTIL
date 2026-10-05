fn main() {
    if std::env::var("DOCS_RS").is_ok() {
        return;
    }
    println!("cargo:rerun-if-changed=include/sentil.h");
    println!("cargo:rerun-if-changed=tests");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/libsentil.dylib");
    }
}