//! The Windows folder and file picker (IFileOpenDialog), called straight
//! through COM. This replaced a PowerShell helper: starting PowerShell with an
//! encoded script and a bypassed execution policy is what malware does, and
//! antivirus tools flagged OOTRR for it.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::null_mut;

pub enum Outcome {
    Chosen(PathBuf),
    Cancelled,
    Failed(String),
}

#[repr(C)]
struct Guid {
    a: u32,
    b: u16,
    c: u16,
    d: [u8; 8],
}

const CLSID_FILE_OPEN_DIALOG: Guid = Guid { a: 0xDC1C_5A9C, b: 0xE88A, c: 0x4DDE, d: [0xA5, 0xA1, 0x60, 0xF8, 0x2A, 0x20, 0xAE, 0xF7] };
const IID_FILE_DIALOG: Guid = Guid { a: 0x42F8_5136, b: 0xDB7E, c: 0x439C, d: [0x85, 0xF1, 0xE4, 0x07, 0x5D, 0x13, 0x5F, 0xC8] };
const IID_SHELL_ITEM: Guid = Guid { a: 0x4382_6D1E, b: 0xE718, c: 0x42EE, d: [0xBC, 0x55, 0xA1, 0xE2, 0x61, 0xC3, 0x7B, 0xFE] };

const COINIT_APARTMENTTHREADED: u32 = 0x2;
const COINIT_DISABLE_OLE1DDE: u32 = 0x4;
const CLSCTX_INPROC_SERVER: u32 = 0x1;
const FOS_PICKFOLDERS: u32 = 0x20;
const FOS_FORCEFILESYSTEM: u32 = 0x40;
const FOS_PATHMUSTEXIST: u32 = 0x800;
const FOS_FILEMUSTEXIST: u32 = 0x1000;
const SIGDN_FILESYSPATH: u32 = 0x8005_8000;
const CANCELLED: i32 = 0x8007_04C7_u32 as i32;
const WS_POPUP: u32 = 0x8000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_EX_TOPMOST: u32 = 0x8;
const WS_EX_TOOLWINDOW: u32 = 0x80;
const WS_EX_LAYERED: u32 = 0x8_0000;

// Slots in the IFileDialog vtable (IUnknown, IModalWindow, then its own).
const RELEASE: usize = 2;
const SHOW: usize = 3;
const SET_OPTIONS: usize = 9;
const GET_OPTIONS: usize = 10;
const SET_FOLDER: usize = 12;
const SET_TITLE: usize = 17;
const GET_RESULT: usize = 20;
// Slot in the IShellItem vtable.
const GET_DISPLAY_NAME: usize = 5;

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, coinit: u32) -> i32;
    fn CoUninitialize();
    fn CoCreateInstance(clsid: *const Guid, outer: *mut c_void, ctx: u32, iid: *const Guid, out: *mut *mut c_void) -> i32;
    fn CoTaskMemFree(p: *mut c_void);
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHCreateItemFromParsingName(path: *const u16, bind: *mut c_void, iid: *const Guid, out: *mut *mut c_void) -> i32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn CreateWindowExW(ex: u32, class: *const u16, name: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32, parent: isize, menu: isize, inst: isize, param: *mut c_void) -> isize;
    fn DestroyWindow(w: isize) -> i32;
    fn SetForegroundWindow(w: isize) -> i32;
    fn GetSystemMetrics(i: i32) -> i32;
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// A COM interface pointer that is released when dropped.
struct Com(*mut c_void);

impl Com {
    /// The function pointer in vtable slot `i`.
    unsafe fn slot(&self, i: usize) -> usize {
        unsafe { *(*(self.0 as *const *const usize)).add(i) }
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let release: extern "system" fn(*mut c_void) -> u32 = std::mem::transmute(self.slot(RELEASE));
                release(self.0);
            }
        }
    }
}

fn hr(what: &str, h: i32) -> Result<(), String> {
    if h < 0 { Err(format!("{what} failed (0x{:08X})", h as u32)) } else { Ok(()) }
}

/// A hidden topmost window that owns the dialog, so the dialog opens in front
/// of the browser that asked for it.
struct Owner(isize);

impl Owner {
    fn new() -> Owner {
        let class: Vec<u16> = wide("STATIC");
        let empty: Vec<u16> = wide("");
        let (cx, cy): (i32, i32) = unsafe { (GetSystemMetrics(0) / 2, GetSystemMetrics(1) / 2) };
        let w: isize = unsafe { CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED, class.as_ptr(), empty.as_ptr(), WS_POPUP | WS_VISIBLE, cx, cy, 1, 1, 0, 0, 0, null_mut()) };
        if w != 0 {
            unsafe { SetForegroundWindow(w) };
        }
        Owner(w)
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe { DestroyWindow(self.0) };
        }
    }
}

fn run(start: &str, title: &str, file: bool, dry: bool) -> Result<Outcome, String> {
    let mut p: *mut c_void = null_mut();
    hr("CoCreateInstance", unsafe { CoCreateInstance(&CLSID_FILE_OPEN_DIALOG, null_mut(), CLSCTX_INPROC_SERVER, &IID_FILE_DIALOG, &mut p) })?;
    let d: Com = Com(p);
    unsafe {
        let get_options: extern "system" fn(*mut c_void, *mut u32) -> i32 = std::mem::transmute(d.slot(GET_OPTIONS));
        let set_options: extern "system" fn(*mut c_void, u32) -> i32 = std::mem::transmute(d.slot(SET_OPTIONS));
        let mut o: u32 = 0;
        hr("GetOptions", get_options(d.0, &mut o))?;
        let more: u32 = if file { FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST | FOS_FILEMUSTEXIST } else { FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST };
        hr("SetOptions", set_options(d.0, o | more))?;
        if !title.is_empty() {
            let set_title: extern "system" fn(*mut c_void, *const u16) -> i32 = std::mem::transmute(d.slot(SET_TITLE));
            let t: Vec<u16> = wide(title);
            hr("SetTitle", set_title(d.0, t.as_ptr()))?;
        }
        if !start.is_empty() && std::path::Path::new(start).is_dir() {
            let s: Vec<u16> = wide(start);
            let mut ip: *mut c_void = null_mut();
            if SHCreateItemFromParsingName(s.as_ptr(), null_mut(), &IID_SHELL_ITEM, &mut ip) >= 0 {
                let item: Com = Com(ip);
                let set_folder: extern "system" fn(*mut c_void, *mut c_void) -> i32 = std::mem::transmute(d.slot(SET_FOLDER));
                set_folder(d.0, item.0);
            }
        }
        if dry {
            return Ok(Outcome::Chosen(PathBuf::from("DRY")));
        }
        let show: extern "system" fn(*mut c_void, isize) -> i32 = std::mem::transmute(d.slot(SHOW));
        let shown: i32 = {
            let owner: Owner = Owner::new();
            show(d.0, owner.0)
        };
        if shown == CANCELLED {
            return Ok(Outcome::Cancelled);
        }
        hr("Show", shown)?;
        let get_result: extern "system" fn(*mut c_void, *mut *mut c_void) -> i32 = std::mem::transmute(d.slot(GET_RESULT));
        let mut rp: *mut c_void = null_mut();
        hr("GetResult", get_result(d.0, &mut rp))?;
        let item: Com = Com(rp);
        let name: extern "system" fn(*mut c_void, u32, *mut *mut u16) -> i32 = std::mem::transmute(item.slot(GET_DISPLAY_NAME));
        let mut sp: *mut u16 = null_mut();
        hr("GetDisplayName", name(item.0, SIGDN_FILESYSPATH, &mut sp))?;
        if sp.is_null() {
            return Err("the picker gave no path".into());
        }
        let mut n: usize = 0;
        while *sp.add(n) != 0 {
            n += 1;
        }
        let path: String = String::from_utf16_lossy(std::slice::from_raw_parts(sp, n));
        CoTaskMemFree(sp as *mut c_void);
        Ok(Outcome::Chosen(PathBuf::from(path)))
    }
}

/// Opens the picker for a folder, or with `file` for one file. With `dry` it
/// sets the dialog up and answers `DRY` without showing it.
pub fn pick(start: &str, title: &str, file: bool, dry: bool) -> Outcome {
    let (start, title): (String, String) = (start.to_string(), title.to_string());
    // The dialog needs a single-threaded COM apartment, so it gets a thread.
    let t = std::thread::spawn(move || {
        let init: i32 = unsafe { CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        if init < 0 {
            return Outcome::Failed(format!("CoInitializeEx failed (0x{:08X})", init as u32));
        }
        let r: Outcome = run(&start, &title, file, dry).unwrap_or_else(Outcome::Failed);
        unsafe { CoUninitialize() };
        r
    });
    t.join().unwrap_or_else(|_| Outcome::Failed("the picker thread stopped".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dialog_is_set_up_without_showing() {
        assert!(matches!(pick("", "Test", false, true), Outcome::Chosen(p) if p == PathBuf::from("DRY")));
        assert!(matches!(pick("C:\\", "Test", true, true), Outcome::Chosen(p) if p == PathBuf::from("DRY")));
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn FindWindowW(class: *const u16, name: *const u16) -> isize;
        fn PostMessageW(w: isize, msg: u32, wp: usize, lp: isize) -> i32;
    }

    /// Opens the real dialog titled `title` on the desktop and sends it `msg`
    /// a moment later.
    fn answer(title: &'static str, msg: u32, wp: usize) -> Outcome {
        let closer = std::thread::spawn(move || {
            let t: Vec<u16> = wide(title);
            for _ in 0..100 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let w: isize = unsafe { FindWindowW(std::ptr::null(), t.as_ptr()) };
                if w != 0 {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    unsafe { PostMessageW(w, msg, wp, 0) };
                    return true;
                }
            }
            false
        });
        let r: Outcome = pick("C:\\Windows", title, false, false);
        assert!(closer.join().unwrap(), "the dialog never appeared");
        r
    }

    #[test]
    #[ignore]
    fn the_dialog_shows_and_a_close_is_a_cancel() {
        assert!(matches!(answer("OOTRR picker self-test 1", 0x0010, 0), Outcome::Cancelled));
    }

    #[test]
    #[ignore]
    fn the_dialog_shows_and_ok_picks_the_open_folder() {
        let r: Outcome = answer("OOTRR picker self-test 2", 0x0111, 1);
        assert!(matches!(&r, Outcome::Chosen(p) if p.as_os_str().eq_ignore_ascii_case("C:\\Windows")), "{}", match r {
            Outcome::Chosen(p) => p.display().to_string(),
            Outcome::Cancelled => "cancelled".into(),
            Outcome::Failed(w) => w,
        });
    }
}
