use windows::core::{Error, HRESULT};

use super::failed;

#[test]
fn a_failure_shows_its_hex_code_and_keeps_non_ascii_text() {
    let error = Error::new(HRESULT(0x8007_0005_u32 as i32), "ファイル é");
    let shown = failed("ctx", &error).to_string();
    assert!(shown.contains("0x80070005"), "{shown}");
    assert!(shown.contains("ファイル é"), "{shown}");
}
