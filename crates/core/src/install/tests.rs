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
fn commands_quote_paths_with_spaces_and_parentheses() {
    let exe = Path::new(r"C:\Program Files (x86)\Greyscreen\greyscreen.exe");
    assert_eq!(
        run_command(exe),
        r#""C:\Program Files (x86)\Greyscreen\greyscreen.exe" --hidden"#
    );
    assert!(
        uninstall_command(exe).starts_with(r#""C:\Program Files (x86)\Greyscreen\greyscreen.exe" "#)
    );
    assert_eq!(
        cleanup_command(Path::new(r"C:\Users\Jo (work)\Greyscreen")),
        r#"ping 127.0.0.1 -n 3 >nul & rmdir /s /q "C:\Users\Jo (work)\Greyscreen""#
    );
}

#[test]
fn uninstall_entry_has_the_expected_values() {
    let values = uninstall_values(
        Path::new("/p/greyscreen.exe"),
        Path::new("/p"),
        "1.2.3",
        2049,
        "20261121",
    );
    assert_eq!(value(&values, "DisplayName"), &text("Greyscreen"));
    assert_eq!(value(&values, "DisplayVersion"), &text("1.2.3"));
    assert_eq!(value(&values, "Publisher"), &text("Greyscreen"));
    assert_eq!(value(&values, "DisplayIcon"), &text("/p/greyscreen.exe"));
    assert_eq!(value(&values, "InstallLocation"), &text("/p"));
    assert_eq!(value(&values, "InstallDate"), &text("20261121"));
    assert_eq!(
        value(&values, "UninstallString"),
        &text("\"/p/greyscreen.exe\" --uninstall")
    );
    assert_eq!(
        value(&values, "QuietUninstallString"),
        value(&values, "UninstallString")
    );
    assert_eq!(value(&values, "NoModify"), &RegValue::Number(1));
    assert_eq!(value(&values, "NoRepair"), &RegValue::Number(1));
    assert_eq!(value(&values, "EstimatedSize"), &RegValue::Number(3));
}

#[test]
fn estimated_size_saturates() {
    let values = uninstall_values(
        Path::new("/p/h.exe"),
        Path::new("/p"),
        "1",
        u64::MAX,
        "20261121",
    );
    assert_eq!(value(&values, "EstimatedSize"), &RegValue::Number(u32::MAX));
}

#[test]
fn cleanup_waits_then_removes_the_folder() {
    assert_eq!(
        cleanup_command(Path::new("/p")),
        "ping 127.0.0.1 -n 3 >nul & rmdir /s /q \"/p\""
    );
}

#[test]
fn missing_installed_copy_is_copied_fresh() {
    assert_eq!(install_plan(false, false), InstallPlan::CopyFresh);
    assert_eq!(install_plan(false, true), InstallPlan::CopyFresh);
}

#[test]
fn identical_installed_copy_is_only_relaunched() {
    assert_eq!(install_plan(true, true), InstallPlan::RelaunchInstalled);
}

#[test]
fn different_installed_copy_is_replaced_after_a_quit() {
    assert_eq!(install_plan(true, false), InstallPlan::QuitAndReplace);
}

#[test]
fn wait_until_returns_once_ready() {
    let mut calls = 0;
    let ready = wait_until(Duration::from_secs(1), Duration::from_millis(1), || {
        calls += 1;
        calls == 3
    });
    assert!(ready);
    assert_eq!(calls, 3);
}

#[test]
fn wait_until_gives_up_after_the_timeout() {
    let ready = wait_until(Duration::from_millis(20), Duration::from_millis(5), || {
        false
    });
    assert!(!ready);
}

#[test]
fn instance_event_names_sit_in_the_local_namespace() {
    assert_eq!(
        instance_event_name("greyscreen", "show"),
        "Local\\greyscreen.show"
    );
    assert_eq!(
        instance_event_name("greyscreen", "quit"),
        "Local\\greyscreen.quit"
    );
}

#[test]
fn uninstall_entry_names_are_unique_and_text_values_are_not_empty() {
    let values = uninstall_values(Path::new("/p/h.exe"), Path::new("/p"), "1", 1, "20261121");
    let mut names: Vec<_> = values.iter().map(|(name, _)| *name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), values.len());
    assert!(
        values
            .iter()
            .all(|(_, v)| !matches!(v, RegValue::Text(t) if t.is_empty()))
    );
}

#[test]
fn install_date_is_eight_digits() {
    let date = install_date_today();
    assert_eq!(date.len(), 8);
    assert!(date.bytes().all(|b| b.is_ascii_digit()));
}

#[test]
fn installed_copy_and_opt_out_run_in_place_without_comparing_files() {
    let never = || -> bool { unreachable!("must not compare files") };
    assert_eq!(start_plan(true, false, false, never), StartPlan::RunInPlace);
    assert_eq!(start_plan(false, true, false, never), StartPlan::RunInPlace);
}

#[test]
fn updater_launch_installs_silently_without_comparing_files() {
    let never = || -> bool { unreachable!("must not compare files") };
    assert_eq!(
        start_plan(false, false, true, never),
        StartPlan::InstallSilently
    );
}

#[test]
fn identical_installed_copy_installs_silently() {
    assert_eq!(
        start_plan(false, false, false, || true),
        StartPlan::InstallSilently
    );
}

#[test]
fn a_download_with_no_matching_installed_copy_shows_setup() {
    assert_eq!(
        start_plan(false, false, false, || false),
        StartPlan::ShowSetup
    );
}

#[test]
fn update_flag_is_not_forwarded_to_the_installed_copy() {
    let args = ["--hidden", UPDATE_FLAG, "x"].map(OsString::from);
    assert_eq!(
        without_update_flag(args),
        ["--hidden", "x"].map(OsString::from)
    );
}
