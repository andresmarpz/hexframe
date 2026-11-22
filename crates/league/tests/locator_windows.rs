#![cfg(windows)]
#![allow(clippy::expect_used)]

use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use greyscreen_league::GameLocator;
use greyscreen_league::locator::WindowLocator;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, RegisterClassW,
    TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW,
};
use windows::core::PCWSTR;

const TITLE_VAR: &str = "GREYSCREEN_LOCATOR_HELPER_TITLE_UNITS";
const CLASS: &str = "RiotWindowClass";
const TARGET_PATH_UNITS: usize = 300;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: the arguments come straight from the system message dispatcher.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn run_hidden_window(title_units: usize) {
    let class = wide(CLASS);
    let title = wide(&"t".repeat(title_units));
    // SAFETY: a null module name asks for the handle of this process.
    let module = unsafe { GetModuleHandleW(PCWSTR::null()) }.expect("module handle");
    let window_class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: module.into(),
        lpszClassName: PCWSTR(class.as_ptr()),
        ..Default::default()
    };
    // SAFETY: `window_class` and the class name buffer outlive the call.
    assert_ne!(unsafe { RegisterClassW(&window_class) }, 0);
    // SAFETY: the class and title buffers are NUL terminated and live for the call; no WS_VISIBLE
    // style is set, so the window is never shown.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class.as_ptr()),
            PCWSTR(title.as_ptr()),
            WINDOW_STYLE(0),
            0,
            0,
            100,
            100,
            None,
            None,
            Some(module.into()),
            None,
        )
    }
    .expect("hidden window");
    println!("ready");
    let mut message = MSG::default();
    // SAFETY: `message` is a live local that receives each message.
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
        // SAFETY: `message` was filled by GetMessageW above.
        unsafe {
            TranslateMessage(&message).ok().unwrap_or(());
            DispatchMessageW(&message);
        }
    }
}

#[test]
fn helper_window_process() {
    if let Ok(units) = env::var(TITLE_VAR) {
        run_hidden_window(units.parse().expect("title length"));
        std::process::exit(0);
    }
}

struct Helper {
    child: Child,
}

impl Drop for Helper {
    fn drop(&mut self) {
        self.child.kill().unwrap_or(());
        self.child.wait().map(drop).unwrap_or(());
    }
}

fn long_directory(root: &Path) -> PathBuf {
    let mut dir = root.to_path_buf();
    let mut index = 0;
    while dir.as_os_str().len() + "\\League of Legends.exe".len() < TARGET_PATH_UNITS {
        index += 1;
        dir.push(format!("{index:02}-{}", "d".repeat(40)));
    }
    dir
}

fn spawn_helper(directory: &Path, exe_name: &str, title_units: usize) -> Helper {
    fs::create_dir_all(directory).expect("create directory");
    let exe = directory.join(exe_name);
    fs::copy(env::current_exe().expect("current exe"), &exe).expect("copy test binary");
    let mut child = Command::new(&exe)
        .args(["helper_window_process", "--exact", "--nocapture"])
        .env(TITLE_VAR, title_units.to_string())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn helper");
    let stdout = child.stdout.take().expect("helper stdout");
    let helper = Helper { child };
    let mut lines = BufReader::new(stdout).lines();
    let ready = lines.any(|line| line.map(|text| text.trim() == "ready").unwrap_or(false));
    assert!(ready, "helper window did not start");
    helper
}

#[test]
fn the_walk_skips_a_decoy_in_front_and_survives_the_game_closing() {
    let root = tempfile::tempdir().expect("temp dir");
    let game_dir = long_directory(root.path());
    let decoy_dir = root.path().join("decoy");

    let mut game = spawn_helper(&game_dir, "League of Legends.exe", 10);
    let game_pid = game.child.id();
    let _decoy = spawn_helper(&decoy_dir, "decoy.exe", 1000);
    let mut locator = WindowLocator::new();

    let found = locator.find().map(|(_, pid)| pid);
    assert_eq!(found, Some(game_pid));

    game.child.kill().expect("kill game helper");
    for _ in 0..200 {
        if let Some((_, pid)) = locator.find() {
            assert_eq!(pid, game_pid);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    game.child.wait().expect("reap game helper");
    assert_eq!(locator.find(), None);
}
