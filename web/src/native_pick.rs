
use std::path::PathBuf as PB;
use std::process::{Command, Stdio};

use ootrr_core::json::Json;

use crate::body;
use crate::error::{Fail, Hr};

pub enum Pick {
    Chosen(PB),
    Cancelled,
    Unavailable(String),
}

const WINDOWS_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -ReferencedAssemblies System.Windows.Forms, System.Drawing -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Windows.Forms;
public static class OotrrPick {
  [ComImport, Guid("DC1C5A9C-E88A-4dde-A5A1-60F82A20AEF7")] private class FileOpenDialog {}
  [ComImport, Guid("42f85136-db7e-439c-85f1-e4075d135fc8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
  private interface IFileOpenDialog {
    [PreserveSig] int Show(IntPtr parent);
    void SetFileTypes(uint c, IntPtr f);
    void SetFileTypeIndex(uint i);
    void GetFileTypeIndex(out uint i);
    void Advise(IntPtr e, out uint c);
    void Unadvise(uint c);
    void SetOptions(uint o);
    void GetOptions(out uint o);
    void SetDefaultFolder(IShellItem i);
    void SetFolder(IShellItem i);
    void GetFolder(out IShellItem i);
    void GetCurrentSelection(out IShellItem i);
    void SetFileName([MarshalAs(UnmanagedType.LPWStr)] string n);
    void GetFileName([MarshalAs(UnmanagedType.LPWStr)] out string n);
    void SetTitle([MarshalAs(UnmanagedType.LPWStr)] string t);
    void SetOkButtonLabel([MarshalAs(UnmanagedType.LPWStr)] string t);
    void SetFileNameLabel([MarshalAs(UnmanagedType.LPWStr)] string t);
    void GetResult(out IShellItem i);
  }
  [ComImport, Guid("43826D1E-E718-42EE-BC55-A1E261C37BFE"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
  private interface IShellItem {
    void BindToHandler(IntPtr pbc, ref Guid bhid, ref Guid riid, out IntPtr ppv);
    void GetParent(out IShellItem i);
    void GetDisplayName(uint sigdn, [MarshalAs(UnmanagedType.LPWStr)] out string name);
    void GetAttributes(uint mask, out uint attrs);
    void Compare(IShellItem other, uint hint, out int order);
  }
  [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
  private static extern void SHCreateItemFromParsingName(string path, IntPtr pbc, ref Guid riid, [MarshalAs(UnmanagedType.Interface)] out IShellItem item);
  public static string Run(string start, string title, bool dry, bool file) {
    var d = (IFileOpenDialog)new FileOpenDialog();
    uint o; d.GetOptions(out o);
    d.SetOptions(file ? (o | 0x40 | 0x800 | 0x1000) : (o | 0x20 | 0x40 | 0x800));
    if (!string.IsNullOrEmpty(title)) d.SetTitle(title);
    if (!string.IsNullOrEmpty(start) && System.IO.Directory.Exists(start)) {
      try { Guid g = typeof(IShellItem).GUID; IShellItem si; SHCreateItemFromParsingName(start, IntPtr.Zero, ref g, out si); d.SetFolder(si); } catch {}
    }
    if (dry) return "DRY";
    int hr;
    using (var owner = new Form()) {
      owner.TopMost = true; owner.ShowInTaskbar = false; owner.FormBorderStyle = FormBorderStyle.None;
      owner.Opacity = 0; owner.StartPosition = FormStartPosition.CenterScreen; owner.Size = new System.Drawing.Size(1, 1);
      owner.Show(); owner.Activate();
      hr = d.Show(owner.Handle);
    }
    if (hr != 0) return null;
    IShellItem r; d.GetResult(out r);
    string p; r.GetDisplayName(0x80058000, out p);
    return p;
  }
}
'@
$p = [OotrrPick]::Run($env:OOTRR_PICK_START, $env:OOTRR_PICK_TITLE, $env:OOTRR_PICK_DRY -eq '1', $env:OOTRR_PICK_FILE -eq '1')
if ($p) { [Console]::Out.Write("OK`t" + $p) } else { [Console]::Out.Write("CANCEL") }
"#;

fn quiet(c: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
}

fn encoded(script: &str) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut out: String = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n: u32 = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        for (i, shift) in [18u32, 12, 6, 0].into_iter().enumerate() {
            out.push(if i <= c.len() { A[((n >> shift) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

fn run_windows(start: &str, title: &str, dry: bool, file: bool) -> Pick {
    let mut c: Command = Command::new("powershell.exe");
    c.args(["-NoProfile", "-NonInteractive", "-STA", "-ExecutionPolicy", "Bypass", "-EncodedCommand"])
        .arg(encoded(WINDOWS_SCRIPT))
        .env("OOTRR_PICK_START", start)
        .env("OOTRR_PICK_TITLE", title)
        .env("OOTRR_PICK_DRY", if dry { "1" } else { "0" })
        .env("OOTRR_PICK_FILE", if file { "1" } else { "0" });
    match quiet(&mut c).output() {
        Ok(o) => parse(&String::from_utf8_lossy(&o.stdout), &String::from_utf8_lossy(&o.stderr)),
        Err(_) => Pick::Unavailable("PowerShell could not be started.".into()),
    }
}

fn run_mac(start: &str, title: &str, file: bool) -> Pick {
    let what: &str = if file { "file" } else { "folder" };
    let with_start: String = format!("return \"OK\t\" & POSIX path of (choose {what} with prompt (item 1 of argv) default location (POSIX file (item 2 of argv)))");
    let plain: String = format!("return \"OK\t\" & POSIX path of (choose {what} with prompt (item 1 of argv))");
    let mut c: Command = Command::new("osascript");
    c.args([
        "-e",
        "on run argv",
        "-e",
        "try",
        "-e",
        "if item 2 of argv is not \"\" then",
        "-e",
        &with_start,
        "-e",
        "end if",
        "-e",
        &plain,
        "-e",
        "on error number -128",
        "-e",
        "return \"CANCEL\"",
        "-e",
        "end try",
        "-e",
        "end run",
        title,
        start,
    ]);
    match quiet(&mut c).output() {
        Ok(o) => parse(&String::from_utf8_lossy(&o.stdout), &String::from_utf8_lossy(&o.stderr)),
        Err(_) => Pick::Unavailable("osascript could not be started.".into()),
    }
}

fn run_linux(start: &str, title: &str, file: bool) -> Pick {
    let begin: String = if start.is_empty() { String::new() } else { format!("{}/", start.trim_end_matches('/')) };
    let at: String = if start.is_empty() { ".".into() } else { start.to_string() };
    let tries: [(&str, Vec<String>); 2] = if file {
        [
            ("zenity", vec!["--file-selection".into(), "--file-filter=Game package | *.nup *.nx".into(), format!("--title={title}"), format!("--filename={begin}")]),
            ("kdialog", vec!["--getopenfilename".into(), at, "*.nup *.nx".into(), "--title".into(), title.to_string()]),
        ]
    } else {
        [
            ("zenity", vec!["--file-selection".into(), "--directory".into(), format!("--title={title}"), format!("--filename={begin}")]),
            ("kdialog", vec!["--getexistingdirectory".into(), at, "--title".into(), title.to_string()]),
        ]
    };
    for (prog, args) in tries {
        let mut c: Command = Command::new(prog);
        c.args(&args);
        let Ok(o) = quiet(&mut c).output() else { continue };
        let out: String = String::from_utf8_lossy(&o.stdout).trim_end_matches(['\n', '\r']).to_string();
        return if o.status.success() && !out.is_empty() { Pick::Chosen(PB::from(out)) } else { Pick::Cancelled };
    }
    Pick::Unavailable("No folder picker found. Install zenity or kdialog, or type the path.".into())
}

fn parse(out: &str, err: &str) -> Pick {
    let t: &str = out.trim_start_matches('\u{feff}').trim_end_matches(['\n', '\r']);
    if let Some(p) = t.strip_prefix("OK\t") {
        let p: &str = p.trim_end_matches(['\n', '\r']);
        let p: &str = if p.len() > 1 { p.trim_end_matches('/') } else { p };
        return Pick::Chosen(PB::from(p));
    }
    if t == "CANCEL" {
        return Pick::Cancelled;
    }
    let why: &str = err.lines().find(|l| !l.trim().is_empty()).unwrap_or("the folder picker gave no answer");
    Pick::Unavailable(why.trim().to_string())
}

/// Opens the system picker for a folder, or with `file` for one file.
pub fn pick(start: &str, title: &str, file: bool) -> Pick {
    if cfg!(windows) {
        run_windows(start, title, false, file)
    } else if cfg!(target_os = "macos") {
        run_mac(start, title, file)
    } else {
        run_linux(start, title, file)
    }
}

pub fn api(lan: bool, b: &Json) -> Hr<Json> {
    if lan {
        return Err(Fail::new(501, "The system folder picker only opens on the computer running OOTRR."));
    }
    let start: &str = body::opt_text(b, "start")?.unwrap_or("");
    let title: &str = body::opt_text(b, "title")?.unwrap_or("Pick a folder");
    if start.len() > 4096 || title.len() > 200 || title.chars().any(char::is_control) || start.chars().any(char::is_control) {
        return Err(Fail::bad("start or title is not allowed"));
    }
    let file: bool = b.get("file").and_then(|x| x.as_bool()) == Some(true);
    match pick(start, title, file) {
        Pick::Chosen(p) if (if file { p.is_file() } else { p.is_dir() }) => Ok(Json::obj().with("path", crate::pathsafe::text(&p).into())),
        Pick::Chosen(_) => Err(Fail::new(422, if file { "The picked file could not be found." } else { "The picked folder could not be found." })),
        Pick::Cancelled => Ok(Json::obj().with("cancelled", true.into())),
        Pick::Unavailable(why) => Err(Fail::new(501, &format!("The system folder picker is not available: {why}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_are_parsed() {
        assert!(matches!(parse("OK\tC:\\Games\\mods\r\n", ""), Pick::Chosen(p) if p == PB::from("C:\\Games\\mods")));
        assert!(matches!(parse("OK\t/Users/me/mods/\n", ""), Pick::Chosen(p) if p == PB::from("/Users/me/mods")));
        assert!(matches!(parse("CANCEL", ""), Pick::Cancelled));
        assert!(matches!(parse("", "boom\nmore"), Pick::Unavailable(w) if w == "boom"));
    }

    #[test]
    fn scripts_are_encoded_as_utf16_base64() {
        assert_eq!(encoded("a"), "YQA=");
        assert_eq!(encoded("ab"), "YQBiAA==");
        assert_eq!(encoded("abc"), "YQBiAGMA");
    }

    #[test]
    fn lan_mode_never_opens_a_dialog() {
        let b: Json = Json::obj();
        assert_eq!(api(true, &b).unwrap_err().status, 501);
    }

    #[test]
    fn the_windows_dialog_code_compiles() {
        if !cfg!(windows) {
            return;
        }
        assert!(matches!(run_windows("", "Test", true, false), Pick::Chosen(p) if p == PB::from("DRY")));
        assert!(matches!(run_windows("", "Test", true, true), Pick::Chosen(p) if p == PB::from("DRY")));
    }
}
