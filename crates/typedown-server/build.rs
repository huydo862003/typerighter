fn main() {
    println!(
        "cargo:rustc-env=BUILD_TIMESTAMP={}",
        std::process::Command::new("node")
            .args(["-e", "process.stdout.write(new Date().toLocaleString())"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
}
