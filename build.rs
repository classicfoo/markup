fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed=markup.rc");
    println!("cargo:rerun-if-changed=icons/markup-icon4.ico");

    embed_resource::compile("markup.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
