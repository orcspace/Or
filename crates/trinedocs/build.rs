use std::{path::PathBuf, process::Command};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=assets/trinedocs.ico");
    println!("cargo:rerun-if-changed=assets/windows.manifest");
    let target = std::env::var("TARGET")?;
    if !target.contains("windows") {
        return Ok(());
    }
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let output = PathBuf::from(std::env::var("OUT_DIR")?);
    let rc = output.join("trinedocs.rc");
    let icon = root
        .join("assets/trinedocs.ico")
        .display()
        .to_string()
        .replace('\\', "\\\\");
    let manifest = root
        .join("assets/windows.manifest")
        .display()
        .to_string()
        .replace('\\', "\\\\");
    let version = std::env::var("CARGO_PKG_VERSION")?;
    let numeric = version
        .split(['.', '-'])
        .take(3)
        .map(|part| part.parse::<u16>().unwrap_or(0).to_string())
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        r#"1 ICON "{icon}"
1 24 "{manifest}"
1 VERSIONINFO
FILEVERSION {numeric},0
PRODUCTVERSION {numeric},0
FILEOS 0x40004L
FILETYPE 0x1L
BEGIN
BLOCK "StringFileInfo"
BEGIN
BLOCK "040904B0"
BEGIN
VALUE "CompanyName", "orcspace\0"
VALUE "FileDescription", "TrineDocs Office\0"
VALUE "FileVersion", "{version}\0"
VALUE "ProductName", "TrineDocs\0"
VALUE "ProductVersion", "{version}\0"
END
END
BLOCK "VarFileInfo"
BEGIN
VALUE "Translation", 0x0409, 1200
END
END
"#
    );
    std::fs::write(&rc, source)?;
    let resource = output.join(if target.ends_with("msvc") {
        "trinedocs.res"
    } else {
        "trinedocs.o"
    });
    let status = if target.ends_with("msvc") {
        Command::new(std::env::var("RC").unwrap_or_else(|_| "rc.exe".into()))
            .arg("/nologo")
            .arg(format!("/fo{}", resource.display()))
            .arg(&rc)
            .status()?
    } else {
        Command::new(
            std::env::var("WINDRES").unwrap_or_else(|_| "x86_64-w64-mingw32-windres".into()),
        )
        .args(["-i"])
        .arg(&rc)
        .args(["-o"])
        .arg(&resource)
        .status()?
    };
    if !status.success() {
        return Err("Resource compiler failed".into());
    }
    println!("cargo:rustc-link-arg-bins={}", resource.display());
    Ok(())
}
