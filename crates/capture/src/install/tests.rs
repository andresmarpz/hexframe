use super::*;

const ROUNDS: usize = 2_000;

#[test]
fn string_property_owns_a_com_buffer_that_drops_once() {
    for _ in 0..ROUNDS {
        let value = string_property(APP_ID).unwrap();
        assert_eq!(value.vt(), VT_LPWSTR);
        assert_eq!(value.to_string(), APP_ID);
        let copy = value.clone();
        assert_eq!(copy, value);
    }
}

#[test]
fn shortcut_writes_repeat_without_heap_corruption() {
    let dir = std::env::temp_dir().join(format!("greyscreen-shortcut-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("greyscreen.exe");
    let lnk = dir.join("Greyscreen.lnk");
    // SAFETY: the matching CoUninitialize runs at the end of this test on the same thread.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .unwrap();
    for _ in 0..200 {
        write_shortcut(&exe, &dir, &lnk).unwrap();
    }
    // SAFETY: balances the successful CoInitializeEx above.
    unsafe { CoUninitialize() };
    fs::remove_dir_all(&dir).ok();
}
