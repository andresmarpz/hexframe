use std::cell::RefCell;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!(
        "greyscreen-delete-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn recording(output: &Path, id: &str) -> PathBuf {
    let folder = output.join(id);
    fs::create_dir_all(folder.join("clips")).expect("folder");
    fs::write(folder.join(META_FILE), "{}").expect("meta");
    fs::write(folder.join("game.mp4"), "video").expect("video");
    folder
}

#[test]
fn a_recording_folder_moves_to_the_bin() {
    let output = temp_dir("move");
    let bin = temp_dir("bin");
    let folder = recording(&output, "2025-01-01_Ahri");
    let mover = |path: &Path| move_into(path, &bin);
    delete_recording(&output, "2025-01-01_Ahri", None, &mover).expect("delete");
    assert!(!folder.exists());
    assert!(bin.join("2025-01-01_Ahri/game.mp4").is_file());
}

#[test]
fn a_name_that_exists_in_the_bin_gets_a_number() {
    let output = temp_dir("collide");
    let bin = temp_dir("collide-bin");
    fs::create_dir_all(bin.join("2025-01-01_Ahri")).expect("old copy");
    recording(&output, "2025-01-01_Ahri");
    move_into(&output.join("2025-01-01_Ahri"), &bin).expect("move");
    assert!(bin.join("2025-01-01_Ahri 2/game.mp4").is_file());
}

#[test]
fn a_folder_outside_the_output_dir_is_refused() {
    let output = temp_dir("inside");
    let elsewhere = temp_dir("outside");
    let outside = recording(&elsewhere, "2025-01-01_Zed");
    let called = RefCell::new(false);
    let mover = |_: &Path| {
        *called.borrow_mut() = true;
        Ok(())
    };
    let escape = format!(
        "../{}/2025-01-01_Zed",
        elsewhere.file_name().unwrap().to_string_lossy()
    );
    for id in [
        escape.as_str(),
        "..",
        ".",
        "",
        "a/b",
        outside.to_str().unwrap(),
    ] {
        let result = delete_recording(&output, id, None, &mover);
        assert!(
            matches!(result, Err(DeleteError::NotARecording)),
            "{id:?}: {result:?}"
        );
    }
    assert!(!*called.borrow());
    assert!(outside.join("game.mp4").is_file());
}

#[test]
fn a_folder_without_game_json_is_refused() {
    let output = temp_dir("no-meta");
    let folder = output.join("holiday-photos");
    fs::create_dir_all(&folder).expect("folder");
    fs::write(folder.join("a.jpg"), "x").expect("file");
    let mover = |_: &Path| panic!("must not move");
    let result = delete_recording(&output, "holiday-photos", None, &mover);
    assert!(matches!(result, Err(DeleteError::NotARecording)));
    assert!(folder.join("a.jpg").is_file());
}

#[cfg(unix)]
#[test]
fn a_symlink_to_a_folder_outside_is_refused() {
    let output = temp_dir("link");
    let elsewhere = temp_dir("link-target");
    let target = recording(&elsewhere, "2025-01-01_Zed");
    std::os::unix::fs::symlink(&target, output.join("sneaky")).expect("symlink");
    let mover = |_: &Path| panic!("must not move");
    let result = delete_recording(&output, "sneaky", None, &mover);
    assert!(matches!(result, Err(DeleteError::NotARecording)));
    assert!(target.join("game.mp4").is_file());
}

#[test]
fn the_recording_in_progress_is_refused() {
    let output = temp_dir("active");
    let folder = recording(&output, "2025-01-01_Ahri");
    let mover = |_: &Path| panic!("must not move");
    let result = delete_recording(&output, "2025-01-01_Ahri", Some("2025-01-01_Ahri"), &mover);
    assert!(matches!(result, Err(DeleteError::InProgress)));
    assert!(folder.is_dir());
}

#[test]
fn running_a_delete_reports_a_notice_and_the_new_library() {
    let output = temp_dir("run");
    let bin = temp_dir("run-bin");
    recording(&output, "2025-01-01_Ahri");
    let seen: Arc<Mutex<Vec<UiMsg>>> = Arc::default();
    let log = seen.clone();
    let sink: Sink = Arc::new(move |msg| log.lock().expect("lock").push(msg));
    let mover = |path: &Path| move_into(path, &bin);
    run(&output, "2025-01-01_Ahri", None, &mover, &sink);
    let seen = seen.lock().expect("lock");
    assert!(matches!(
        &seen[0],
        UiMsg::Notice {
            level: NoticeLevel::Info,
            ..
        }
    ));
    assert!(matches!(&seen[1], UiMsg::Library(entries) if entries.is_empty()));
    assert!(!output.join("2025-01-01_Ahri").exists());
}

#[test]
fn a_refused_delete_reports_an_error_and_keeps_the_library() {
    let output = temp_dir("refuse");
    recording(&output, "2025-01-01_Ahri");
    let seen: Arc<Mutex<Vec<UiMsg>>> = Arc::default();
    let log = seen.clone();
    let sink: Sink = Arc::new(move |msg| log.lock().expect("lock").push(msg));
    let mover = |_: &Path| panic!("must not move");
    run(&output, "../escape", None, &mover, &sink);
    let seen = seen.lock().expect("lock");
    assert!(matches!(
        &seen[0],
        UiMsg::Notice {
            level: NoticeLevel::Error,
            ..
        }
    ));
    assert!(output.join("2025-01-01_Ahri").is_dir());
}

fn refuse_while_open(handle_open: &AtomicBool, bin: &Path) -> impl Fn(&Path) -> io::Result<()> {
    move |path: &Path| {
        if handle_open.load(Ordering::SeqCst) {
            Err(io::Error::new(io::ErrorKind::ResourceBusy, "in use"))
        } else {
            move_into(path, bin)
        }
    }
}

#[test]
fn a_delete_waits_for_a_handle_that_closes_soon() {
    let output = temp_dir("busy-soon");
    let bin = temp_dir("busy-soon-bin");
    let folder = recording(&output, "2025-01-01_Ahri");
    let handle_open = Arc::new(AtomicBool::new(true));
    let closer = handle_open.clone();
    let closing = thread::spawn(move || {
        thread::sleep(IN_USE_DELAY * 3);
        closer.store(false, Ordering::SeqCst);
    });
    let mover = refuse_while_open(&handle_open, &bin);
    delete_recording(&output, "2025-01-01_Ahri", None, &mover).expect("delete");
    closing.join().expect("closer");
    assert!(!folder.exists());
}

#[test]
fn a_handle_that_stays_open_reports_the_file_is_in_use() {
    let output = temp_dir("busy-stays");
    let bin = temp_dir("busy-stays-bin");
    let folder = recording(&output, "2025-01-01_Ahri");
    let handle_open = AtomicBool::new(true);
    let seen: Arc<Mutex<Vec<UiMsg>>> = Arc::default();
    let log = seen.clone();
    let sink: Sink = Arc::new(move |msg| log.lock().expect("lock").push(msg));
    let mover = refuse_while_open(&handle_open, &bin);
    run(&output, "2025-01-01_Ahri", None, &mover, &sink);
    let seen = seen.lock().expect("lock");
    assert!(matches!(
        &seen[0],
        UiMsg::Notice { level: NoticeLevel::Error, text } if text == "Couldn't delete: the file is in use"
    ));
    assert!(folder.join("game.mp4").is_file());
}
