use super::*;

fn value<'a>(values: &'a [(&'static str, RegValue)], name: &str) -> &'a RegValue {
    &values.iter().find(|(key, _)| *key == name).unwrap().1
}

fn text(value: &str) -> RegValue {
    RegValue::Text(value.to_owned())
}

#[test]
fn exe_inside_the_install_dir_is_an_installed_copy() {
    let dir = Path::new("/users/a/appdata/local/programs/greyscreen");
    assert!(is_inside(&dir.join("greyscreen.exe"), dir));
}

#[test]
fn exe_elsewhere_is_not_an_installed_copy() {
    let dir = Path::new("/users/a/appdata/local/programs/greyscreen");
    assert!(!is_inside(
        Path::new("/users/a/downloads/greyscreen.exe"),
        dir
    ));
    assert!(!is_inside(
        Path::new("/users/a/appdata/local/programs/greyscreen-old/greyscreen.exe"),
        dir
    ));
}

#[test]
fn path_check_ignores_case() {
    let dir = Path::new("/Users/A/AppData/Local/Programs/Greyscreen");
    assert!(is_inside(
        Path::new("/users/a/appdata/local/programs/greyscreen/greyscreen.exe"),
        dir
    ));
}

#[test]
fn run_command_quotes_the_exe_and_starts_hidden() {
    assert_eq!(
        run_command(Path::new("/p q/greyscreen.exe")),
        "\"/p q/greyscreen.exe\" --hidden"
    );
}

#[test]
fn uninstall_entry_has_the_expected_values() {
    let values = uninstall_values(Path::new("/p/greyscreen.exe"), Path::new("/p"), "1.2.3", 2049);
    assert_eq!(value(&values, "DisplayName"), &text("Greyscreen"));
    assert_eq!(value(&values, "DisplayVersion"), &text("1.2.3"));
    assert_eq!(value(&values, "Publisher"), &text("Greyscreen"));
    assert_eq!(value(&values, "DisplayIcon"), &text("/p/greyscreen.exe"));
    assert_eq!(value(&values, "InstallLocation"), &text("/p"));
    assert_eq!(
        value(&values, "UninstallString"),
        &text("\"/p/greyscreen.exe\" --uninstall")
    );
    assert_eq!(value(&values, "NoModify"), &RegValue::Number(1));
    assert_eq!(value(&values, "NoRepair"), &RegValue::Number(1));
    assert_eq!(value(&values, "EstimatedSize"), &RegValue::Number(3));
}

#[test]
fn estimated_size_saturates() {
    let values = uninstall_values(Path::new("/p/h.exe"), Path::new("/p"), "1", u64::MAX);
    assert_eq!(value(&values, "EstimatedSize"), &RegValue::Number(u32::MAX));
}

#[test]
fn cleanup_waits_then_removes_the_folder() {
    assert_eq!(
        cleanup_command(Path::new("/p")),
        "ping 127.0.0.1 -n 3 >nul & rmdir /s /q \"/p\""
    );
}
