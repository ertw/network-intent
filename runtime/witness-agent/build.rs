fn main() {
    if std::env::var_os("CARGO_FEATURE_NATIVE_UBUS").is_none() {
        return;
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        panic!(
            "native-ubus is supported only on Linux/OpenWrt; do not emulate it on this platform"
        );
    }
    println!("cargo:rerun-if-changed=native/ubus_readonly.c");
    cc::Build::new()
        .file("native/ubus_readonly.c")
        .flag_if_supported("-Werror")
        .compile("intent_ubus_readonly");
    println!("cargo:rustc-link-lib=ubus");
    println!("cargo:rustc-link-lib=ubox");
    // blobmsg_{format,add}_json live in their own OpenWrt shared library.
    println!("cargo:rustc-link-lib=blobmsg_json");
    println!("cargo:rustc-link-lib=json-c");
}
