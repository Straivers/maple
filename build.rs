fn main() {
    config_platform();
}

#[cfg(target_os = "windows")]
fn config_platform() {
    embed_resource::compile("resources/app-manifest.rc");
}

#[cfg(not(target_os = "windows"))]
fn config_platform() {}
