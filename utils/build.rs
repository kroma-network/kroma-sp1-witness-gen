use std::{ fs::File, io::Write };

fn parse_package_version(package_data: &str, package_name: &str) -> Option<String> {
    let name_key = format!(r#"name = "{}""#, package_name);
    if let Some(name_pos) = package_data.find(&name_key) {
        if let Some(version_pos) = package_data[name_pos..].find("version = ") {
            let version_start = name_pos + version_pos + "version = ".len() + 1;
            if let Some(version_end) = package_data[version_start..].find('"') {
                return Some(package_data[version_start..version_start + version_end].to_string());
            }
        }
    }
    None
}

fn main() {
    // Open Cargo.lock
    let cargo_toml = std::fs::read_to_string("../Cargo.lock").unwrap();

    // Find the version of `sp1-sdk``
    let sp1_sdk_version = parse_package_version(&cargo_toml, "sp1-sdk").unwrap();

    // Save the version to `src/deps_version.rs`
    let mut file = File::create("src/deps_version.rs").unwrap();
    write!(file, "pub const SP1_SDK_VERSION: &str = \"v{}\";", sp1_sdk_version).unwrap();
}
