fn main() -> std::io::Result<()> {
    print_build_version();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_manifest_file("app.manifest");
        res.set_icon("assets/greyscreen.ico");
        res.set("ProductName", "Greyscreen");
        res.set("FileDescription", "Greyscreen");
        res.set("CompanyName", "");
        res.compile()?;
    }
    Ok(())
}

fn git(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn print_build_version() {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let build = match git(&["rev-parse", "--short=8", "HEAD"]) {
        Some(sha) => {
            let dirty = git(&["status", "--porcelain"]).is_some_and(|s| !s.is_empty());
            format!("{version}+{sha}{}", if dirty { "-dirty" } else { "" })
        }
        None => version,
    };
    println!("cargo:rustc-env=GREYSCREEN_BUILD={build}");
    for path in ["HEAD", "refs/heads", "index"] {
        if let Some(p) = git(&["rev-parse", "--git-path", path]) {
            println!("cargo:rerun-if-changed={p}");
        }
    }
    println!("cargo:rerun-if-changed=src");
}
