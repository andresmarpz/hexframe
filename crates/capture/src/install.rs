use std::fs;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use greyscreen_core::install::{
    APP_ID, EXE_NAME, INSTALL_DIR_NAME, InstallPlan, NO_INSTALL_ENV, QUIT_POLL, QUIT_WAIT, RUN_KEY,
    RUN_VALUE, RegValue, StartPlan, UNINSTALL_KEY, UPDATE_FLAG, cleanup_command,
    install_date_today, install_plan, is_inside, run_command, start_plan, uninstall_values,
    wait_until, without_update_flag,
};
use windows::Win32::Foundation::{E_OUTOFMEMORY, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
use windows::Win32::System::Com::StructuredStorage::{
    PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemAlloc, CoUninitialize, IPersistFile,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW,
};
use windows::Win32::System::Variant::VT_LPWSTR;
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::{IShellLinkW, SetCurrentProcessExplicitAppUserModelID, ShellLink};
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONINFORMATION, MB_OK, MessageBoxW};
use windows::core::{HSTRING, Interface, PCWSTR, PWSTR};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const SHORTCUT_NAME: &str = "Greyscreen.lnk";

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("cannot find the user's app data folder")]
    NoAppData,
    #[error("cannot copy the app to {path}: {source}")]
    Copy { path: PathBuf, source: io::Error },
    #[error("Greyscreen is still running. Quit it from the tray, then run this file again.")]
    StillRunning,
    #[error("the shortcut thread stopped")]
    ShortcutThread,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Windows(#[from] windows::core::Error),
}

fn programs_dir() -> Result<PathBuf, InstallError> {
    let local = std::env::var_os("LOCALAPPDATA").ok_or(InstallError::NoAppData)?;
    Ok(PathBuf::from(local).join("Programs"))
}

pub fn install_dir() -> Result<PathBuf, InstallError> {
    Ok(programs_dir()?.join(INSTALL_DIR_NAME))
}

fn shortcut_path() -> Result<PathBuf, InstallError> {
    let roaming = std::env::var_os("APPDATA").ok_or(InstallError::NoAppData)?;
    Ok(PathBuf::from(roaming)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join(SHORTCUT_NAME))
}

pub fn is_installed_copy() -> bool {
    let (Ok(exe), Ok(dir)) = (std::env::current_exe(), install_dir()) else {
        return false;
    };
    is_inside(&exe, &dir)
}

pub fn set_process_app_id() {
    // SAFETY: the id is a valid string that outlives the call.
    if let Err(error) = unsafe { SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(APP_ID)) } {
        tracing::warn!(%error, "cannot set the app user model id");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    ClosingRunning,
    Copying,
}

pub fn current_start_plan() -> StartPlan {
    let (Ok(exe), Ok(dir)) = (std::env::current_exe(), install_dir()) else {
        return StartPlan::RunInPlace;
    };
    start_plan(
        is_inside(&exe, &dir),
        std::env::var_os(NO_INSTALL_ENV).is_some(),
        std::env::args().any(|arg| arg == UPDATE_FLAG),
        || files_match(&exe, &dir.join(EXE_NAME)),
    )
}

pub fn install(mut progress: impl FnMut(Progress)) -> Result<(), InstallError> {
    let source = std::env::current_exe()?;
    let dir = install_dir()?;
    let target = dir.join(EXE_NAME);
    fs::create_dir_all(&dir)?;
    let identical = files_match(&source, &target);
    let plan = install_plan(target.exists(), identical);
    if plan == InstallPlan::QuitAndReplace {
        tracing::info!("an installed copy exists, asking a running one to quit");
        progress(Progress::ClosingRunning);
        if crate::instance::request_quit(APP_ID) {
            tracing::info!("quit signalled, waiting for the installed exe to unlock");
        }
        if !wait_until(QUIT_WAIT, QUIT_POLL, || is_writable(&target)) {
            tracing::warn!("the installed exe is still locked, giving up");
            return Err(InstallError::StillRunning);
        }
        tracing::info!("the installed exe is unlocked");
        wait_until(QUIT_WAIT, QUIT_POLL, || {
            !crate::instance::is_running(APP_ID)
        });
    }
    progress(Progress::Copying);
    let size = if plan == InstallPlan::RelaunchInstalled {
        fs::metadata(&target)?.len()
    } else {
        fs::copy(&source, &target).map_err(|source| InstallError::Copy {
            path: target.clone(),
            source,
        })?
    };
    tracing::info!(size, "copied the exe");
    match write_uninstall_entry(&target, &dir, size) {
        Ok(()) => tracing::info!("wrote the uninstall entry"),
        Err(error) => tracing::warn!(%error, "cannot write the uninstall entry"),
    }
    match create_shortcut(&target, &dir) {
        Ok(()) => tracing::info!("wrote the Start menu shortcut"),
        Err(error) => tracing::warn!(%error, "cannot write the Start menu shortcut"),
    }
    tracing::info!("installed");
    Ok(())
}

pub fn install_and_relaunch() -> Result<bool, InstallError> {
    let source = std::env::current_exe()?;
    let dir = install_dir()?;
    if is_inside(&source, &dir) {
        return Ok(false);
    }
    if files_match(&source, &dir.join(EXE_NAME)) {
        tracing::info!("the installed copy is identical, relaunching it");
        launch_installed(without_update_flag(std::env::args_os().skip(1)))?;
        return Ok(true);
    }
    match install(drop) {
        Ok(()) => {}
        Err(error @ InstallError::StillRunning) => {
            show_still_running(&error);
            return Ok(true);
        }
        Err(err) => return Err(err),
    }
    tracing::info!("relaunching the installed copy");
    launch_installed(without_update_flag(std::env::args_os().skip(1)))?;
    Ok(true)
}

pub fn launch_installed(args: impl IntoIterator<Item = std::ffi::OsString>) -> io::Result<()> {
    let dir = install_dir().map_err(|err| io::Error::other(err.to_string()))?;
    Command::new(dir.join(EXE_NAME))
        .args(args)
        .current_dir(&dir)
        .spawn()
        .map(drop)
}

pub fn launch_here_without_installing() -> io::Result<()> {
    Command::new(std::env::current_exe()?)
        .env(NO_INSTALL_ENV, "1")
        .spawn()
        .map(drop)
}

fn files_match(a: &Path, b: &Path) -> bool {
    let (Ok(meta_a), Ok(meta_b)) = (fs::metadata(a), fs::metadata(b)) else {
        return false;
    };
    meta_a.len() == meta_b.len() && matches!((fs::read(a), fs::read(b)), (Ok(x), Ok(y)) if x == y)
}

fn is_writable(path: &Path) -> bool {
    !path.exists() || fs::OpenOptions::new().write(true).open(path).is_ok()
}

fn show_still_running(error: &InstallError) {
    // SAFETY: both strings outlive the call and no owner window is used.
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(error.to_string()),
            &HSTRING::from("Greyscreen"),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

pub fn uninstall() {
    if crate::instance::request_quit(APP_ID) {
        tracing::info!("asked the running copy to quit");
        let stopped = wait_until(QUIT_WAIT, QUIT_POLL, || {
            !crate::instance::is_running(APP_ID)
        });
        tracing::info!(stopped, "waited for the running copy");
    }
    if let Ok(shortcut) = shortcut_path() {
        fs::remove_file(shortcut).ok();
    }
    delete_key(UNINSTALL_KEY);
    set_start_with_windows(false).ok();
    if let Ok(dir) = install_dir() {
        spawn_cleanup(&dir);
    }
}

fn spawn_cleanup(dir: &Path) {
    Command::new("cmd")
        .arg("/c")
        .raw_arg(cleanup_command(dir))
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok();
}

pub fn start_with_windows() -> bool {
    start_with_windows_value(RUN_VALUE)
}

fn start_with_windows_value(value: &str) -> bool {
    let Ok(key) = Key::open(RUN_KEY, KEY_READ) else {
        return false;
    };
    let name = HSTRING::from(value);
    // SAFETY: the key is open and the name outlives the call; no data is requested.
    let status = unsafe { RegQueryValueExW(key.0, &name, None, None, None, None) };
    status == ERROR_SUCCESS
}

pub fn set_start_with_windows(enabled: bool) -> Result<(), InstallError> {
    if enabled {
        let exe = install_dir()?.join(EXE_NAME);
        let key = Key::create(RUN_KEY)?;
        key.set(RUN_VALUE, &RegValue::Text(run_command(&exe)))
    } else {
        delete_run_value(RUN_VALUE)
    }
}

fn delete_run_value(value: &str) -> Result<(), InstallError> {
    let key = Key::open(RUN_KEY, KEY_SET_VALUE)?;
    let name = HSTRING::from(value);
    // SAFETY: the key is open and the name outlives the call.
    let status = unsafe { RegDeleteValueW(key.0, &name) };
    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(windows::core::Error::from(status.to_hresult()).into())
    }
}

fn write_uninstall_entry(exe: &Path, dir: &Path, size: u64) -> Result<(), InstallError> {
    let key = Key::create(UNINSTALL_KEY)?;
    let date = install_date_today();
    for (name, value) in uninstall_values(exe, dir, env!("CARGO_PKG_VERSION"), size, &date) {
        key.set(name, &value)?;
    }
    Ok(())
}

fn delete_key(path: &str) {
    // SAFETY: the path outlives the call.
    unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, &HSTRING::from(path)) }
        .ok()
        .ok();
}

struct Key(HKEY);

impl Key {
    fn create(path: &str) -> Result<Key, InstallError> {
        let mut key = HKEY::default();
        // SAFETY: the path and the out pointer outlive the call; no security attributes are passed.
        unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(path),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key,
                None,
            )
        }
        .ok()?;
        Ok(Key(key))
    }

    fn open(
        path: &str,
        access: windows::Win32::System::Registry::REG_SAM_FLAGS,
    ) -> Result<Key, InstallError> {
        let mut key = HKEY::default();
        // SAFETY: the path and the out pointer outlive the call.
        unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(path),
                None,
                access,
                &mut key,
            )
        }
        .ok()?;
        Ok(Key(key))
    }

    fn set(&self, name: &str, value: &RegValue) -> Result<(), InstallError> {
        let (kind, data) = match value {
            RegValue::Text(text) => (
                REG_SZ,
                text.encode_utf16()
                    .chain(std::iter::once(0))
                    .flat_map(u16::to_le_bytes)
                    .collect::<Vec<u8>>(),
            ),
            RegValue::Number(number) => (REG_DWORD, number.to_le_bytes().to_vec()),
        };
        // SAFETY: the key is open; the name and data outlive the call.
        unsafe { RegSetValueExW(self.0, &HSTRING::from(name), None, kind, Some(&data)) }.ok()?;
        Ok(())
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful open or create and is closed once.
        unsafe { RegCloseKey(self.0) }.ok().ok();
    }
}

fn create_shortcut(exe: &Path, dir: &Path) -> Result<(), InstallError> {
    let lnk = shortcut_path()?;
    if let Some(parent) = lnk.parent() {
        fs::create_dir_all(parent)?;
    }
    let (exe, dir) = (exe.to_owned(), dir.to_owned());
    std::thread::Builder::new()
        .name("greyscreen-shortcut".into())
        .spawn(move || {
            // SAFETY: no pointers are passed; the matching CoUninitialize runs below on this thread.
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok()?;
            let result = write_shortcut(&exe, &dir, &lnk);
            // SAFETY: balances the successful CoInitializeEx above.
            unsafe { CoUninitialize() };
            result
        })?
        .join()
        .map_err(|_| InstallError::ShortcutThread)?
        .map_err(InstallError::from)
}

fn write_shortcut(exe: &Path, dir: &Path, lnk: &Path) -> windows::core::Result<()> {
    let exe = HSTRING::from(exe.as_os_str());
    // SAFETY: COM is initialized on this thread and every argument outlives its call.
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(&exe)?;
        link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))?;
        link.SetDescription(&HSTRING::from("Greyscreen"))?;
        link.SetIconLocation(&exe, 0)?;
        let value = string_property(APP_ID)?;
        let store: IPropertyStore = link.cast()?;
        store.SetValue(&PKEY_AppUserModel_ID, &value)?;
        store.Commit()?;
        let file: IPersistFile = link.cast()?;
        file.Save(&HSTRING::from(lnk.as_os_str()), true)
    }
}

fn string_property(text: &str) -> windows::core::Result<PROPVARIANT> {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = wide.len() * std::mem::size_of::<u16>();
    // SAFETY: the buffer comes from CoTaskMemAlloc because PROPVARIANT's drop frees it with CoTaskMemFree.
    let buffer = unsafe { CoTaskMemAlloc(bytes) }.cast::<u16>();
    if buffer.is_null() {
        return Err(E_OUTOFMEMORY.into());
    }
    // SAFETY: the buffer holds `wide.len()` u16 values and does not overlap the vector.
    unsafe { std::ptr::copy_nonoverlapping(wide.as_ptr(), buffer, wide.len()) };
    Ok(PROPVARIANT {
        Anonymous: PROPVARIANT_0 {
            Anonymous: std::mem::ManuallyDrop::new(PROPVARIANT_0_0 {
                vt: VT_LPWSTR,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: PROPVARIANT_0_0_0 {
                    pwszVal: PWSTR(buffer),
                },
            }),
        },
    })
}
