fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");

    #[cfg(windows)]
    if std::path::Path::new("assets/app.ico").exists() {
        let version = env!("CARGO_PKG_VERSION");
        let file_version = format!("{}.0", version);

        winresource::WindowsResource::new()
            .set_icon("assets/app.ico")
            .set("ProductName", "Game Save Utility")
            .set("FileDescription", "Game Save Utility")
            .set("FileVersion", &file_version)
            .set("ProductVersion", version)
            .compile()
            .expect("failed to compile Windows resources");
    }
}
