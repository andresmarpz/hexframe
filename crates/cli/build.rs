fn main() -> std::io::Result<()> {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../app/assets/greyscreen.ico");
        res.set("ProductName", "Greyscreen");
        res.set("FileDescription", "Greyscreen CLI");
        res.set("CompanyName", "");
        res.compile()?;
    }
    Ok(())
}
