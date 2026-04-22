use anyhow::Result;

pub fn check_java() -> Result<()> {
    let output = std::process::Command::new("java").arg("-version").output();
    match output {
        Ok(out) if out.status.success() => Ok(()),
        _ => anyhow::bail!("Java is not installed or not in PATH. Please install Java."),
    }
}

pub fn get_java_major_version() -> Option<u32> {
    // `java -version` prints to stderr: e.g. `openjdk version "21.0.3" ...`
    let out = std::process::Command::new("java")
        .arg("-version")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stderr);
    // Match patterns like `version "21.0.3"` or `version "1.8.0_xyz"`
    for token in text.split_whitespace() {
        let token = token.trim_matches('"');
        let major: &str = if token.starts_with("1.") {
            // Old-style: 1.8 → major 8
            token.splitn(3, '.').nth(1).unwrap_or("0")
        } else {
            token.splitn(2, '.').next().unwrap_or("0")
        };
        if let Ok(n) = major.parse::<u32>() {
            if n > 0 {
                return Some(n);
            }
        }
    }
    None
}

pub fn require_java_version(min_major: u32) -> Result<()> {
    check_java()?;
    match get_java_major_version() {
        Some(v) if v >= min_major => Ok(()),
        Some(v) => anyhow::bail!(
            "This server requires Java {min_major}+, but Java {v} was found.\n\
             Please install a newer JDK: https://adoptium.net"
        ),
        None => {
            println!("[warn] Could not detect Java version — proceeding anyway.");
            Ok(())
        }
    }
}
