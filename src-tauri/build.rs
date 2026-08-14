fn main() {
    // Load .env from the workspace root (one level up from src-tauri/) at compile time
    // and bake any recognised keys into the binary so they don't depend on a runtime CWD.
    let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".env");

    if let Ok(contents) = std::fs::read_to_string(&env_path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if matches!(key, "VIRUSTOTAL_API_KEY" | "MALWAREBAZAAR_API_KEY" | "NVD_API_KEY") {
                    println!("cargo:rustc-env={key}={value}");
                }
            }
        }
    }

    // Re-run this build script if .env changes.
    println!("cargo:rerun-if-changed=../.env");

    tauri_build::build()
}
