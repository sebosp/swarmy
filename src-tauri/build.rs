/// The libraries whose versions are shown in the Home view, read from the workspace lock file.
const VERSIONED_DEPENDENCIES: [&str; 4] = ["s2protocol", "rerun", "polars", "tauri"];

fn main() {
    println!("cargo:rerun-if-changed=../Cargo.lock");
    let lock_file = std::fs::read_to_string("../Cargo.lock").unwrap_or_default();
    for dependency in VERSIONED_DEPENDENCIES {
        let version = lock_file
            .split("[[package]]")
            .find_map(|package| {
                let mut lines = package.lines().map(str::trim);
                let name = lines.find(|line| line.starts_with("name = "))?;
                if name != format!("name = \"{dependency}\"") {
                    return None;
                }
                let version = lines.find(|line| line.starts_with("version = "))?;
                Some(
                    version
                        .trim_start_matches("version = ")
                        .trim_matches('"')
                        .to_string(),
                )
            })
            .unwrap_or_else(|| "unknown".to_string());
        println!(
            "cargo:rustc-env=SWARMY_{}_VERSION={version}",
            dependency.to_uppercase()
        );
    }
    tauri_build::build()
}
