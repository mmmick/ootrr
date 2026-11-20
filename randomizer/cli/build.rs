//! Gives the Windows exe its version info, manifest and (if there is one)
//! icon. An exe without them looks more like malware to antivirus heuristics,
//! and the details show in Properties > Details and in Task Manager.
//!
//! This needs rc.exe from the Windows SDK, which comes with the Visual Studio
//! C++ build tools and is on the GitHub Windows runners. Without it the build
//! still works and only warns.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RC");
    if !env::var("TARGET").unwrap_or_default().ends_with("windows-msvc") {
        return;
    }
    let root: PathBuf = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..").join("..").join("packaging").join("windows");
    let manifest: PathBuf = root.join("ootrr.manifest");
    let icon: PathBuf = root.join("ootrr.ico");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rerun-if-changed={}", icon.display());
    let Some(rc) = find_rc() else {
        println!("cargo:warning=rc.exe was not found, so ootrr.exe gets no version info or manifest. Install the Windows SDK or set RC.");
        return;
    };
    let out: PathBuf = PathBuf::from(env::var("OUT_DIR").unwrap());
    let script: PathBuf = out.join("ootrr.rc");
    let res: PathBuf = out.join("ootrr.res");
    std::fs::write(&script, rc_script(&manifest, icon.is_file().then_some(&icon))).unwrap();
    let ok: bool = Command::new(&rc).arg("/nologo").arg("/fo").arg(&res).arg(&script).status().map(|s| s.success()).unwrap_or(false);
    if !ok {
        panic!("rc.exe could not compile {}", script.display());
    }
    println!("cargo:rustc-link-arg-bins={}", res.display());
}

fn quoted(p: &Path) -> String {
    format!("\"{}\"", p.display().to_string().replace('\\', "\\\\"))
}

fn rc_script(manifest: &Path, icon: Option<&PathBuf>) -> String {
    let v: String = env::var("CARGO_PKG_VERSION").unwrap();
    let mut n: Vec<u16> = v.split(['.', '-', '+']).take(3).map(|x| x.parse().unwrap_or(0)).collect();
    n.resize(4, 0);
    let num: String = n.iter().map(u16::to_string).collect::<Vec<_>>().join(",");
    let repo: String = env::var("CARGO_PKG_REPOSITORY").unwrap_or_default();
    let mut s: String = String::new();
    if let Some(i) = icon {
        s += &format!("1 ICON {}\n", quoted(i));
    }
    s += &format!("1 24 {}\n", quoted(manifest));
    s += &format!(
        r#"1 VERSIONINFO
FILEVERSION {num}
PRODUCTVERSION {num}
FILEFLAGSMASK 0x3F
FILEFLAGS 0
FILEOS 0x40004
FILETYPE 1
FILESUBTYPE 0
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "OOTRR contributors"
      VALUE "FileDescription", "OOTRR randomizer"
      VALUE "FileVersion", "{v}"
      VALUE "InternalName", "ootrr"
      VALUE "LegalCopyright", "OOTRR contributors. GPL-3.0-or-later."
      VALUE "OriginalFilename", "ootrr.exe"
      VALUE "ProductName", "OOTRR"
      VALUE "ProductVersion", "{v}"
      VALUE "Comments", "Open source: {repo}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    );
    s
}

/// rc.exe from RC, the PATH, or the newest Windows 10/11 SDK.
fn find_rc() -> Option<PathBuf> {
    if let Some(p) = env::var_os("RC").map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(p);
    }
    if Command::new("rc.exe").arg("/?").output().is_ok() {
        return Some(PathBuf::from("rc.exe"));
    }
    let arch: &str = if cfg!(target_arch = "aarch64") { "arm64" } else if cfg!(target_arch = "x86") { "x86" } else { "x64" };
    let kits: PathBuf = PathBuf::from(env::var_os("ProgramFiles(x86)")?).join("Windows Kits").join("10").join("bin");
    let mut vers: Vec<PathBuf> = std::fs::read_dir(&kits).ok()?.flatten().map(|e| e.path()).filter(|p| p.join(arch).join("rc.exe").is_file()).collect();
    vers.sort_by_key(|p| p.file_name().and_then(|n| n.to_str()).map(|n| n.split('.').map(|x| x.parse::<u32>().unwrap_or(0)).collect::<Vec<u32>>()).unwrap_or_default());
    vers.pop().map(|p| p.join(arch).join("rc.exe"))
}
