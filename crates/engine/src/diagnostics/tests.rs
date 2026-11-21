use serde::Serialize;

use super::*;

#[derive(Serialize)]
struct Row {
    n: u32,
}

#[test]
fn writes_one_json_line_per_call_with_its_type() {
    let dir = std::env::temp_dir().join(format!("greyscreen-diag-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let writer = DiagWriter::start(&dir).expect("start");
    writer.line("setup", &Row { n: 1 });
    writer.line("second", &Row { n: 2 });
    drop(writer);
    let text = std::fs::read_to_string(dir.join(DIAG_FILE)).expect("read");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [r#"{"n":1,"type":"setup"}"#, r#"{"n":2,"type":"second"}"#]
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_full_queue_drops_lines_instead_of_blocking() {
    let dir = std::env::temp_dir().join(format!("greyscreen-diag-full-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let writer = DiagWriter::start(&dir).expect("start");
    for n in 0..5_000 {
        writer.line("second", &Row { n });
    }
    drop(writer);
    let lines = std::fs::read_to_string(dir.join(DIAG_FILE))
        .expect("read")
        .lines()
        .count();
    assert!(lines > 0 && lines <= 5_000);
    std::fs::remove_dir_all(&dir).ok();
}
