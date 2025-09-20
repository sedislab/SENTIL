fn main() {
    if std::env::var("DOCS_RS").is_ok() {
        return;
    }
    println!("cargo:rerun-if-changed=include/sentil.h");
    println!("cargo:rerun-if-changed=tests");
}