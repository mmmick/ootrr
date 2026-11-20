
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

fn quiet(c: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
}

fn run_windows(start: &str, title: &str, dry: bool, file: bool) -> Pick {
    use ootrr_os::pick::Outcome;
    match ootrr_os::pick::pick(start, title, file, dry) {
        Outcome::Chosen(p) => Pick::Chosen(p),
        Outcome::Cancelled => Pick::Cancelled,
        Outcome::Failed(why) => Pick::Unavailable(why),
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
