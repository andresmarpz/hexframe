use super::*;

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
