fn main() {
    println!("cargo:rustc-link-lib=dylib=tdjson");
    println!("cargo:rustc-link-search=all=/home/tolik/FuseTelegramFilesystem/src/");
}
