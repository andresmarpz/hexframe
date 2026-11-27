pub(crate) const BASE_URL: &str = "https://api.greyscreen.app";
pub(crate) const BASE_URL_ENV: &str = "GREYSCREEN_API_URL";

pub fn base_url() -> String {
    resolve_base_url(std::env::var(BASE_URL_ENV).ok().as_deref())
}

fn resolve_base_url(from_env: Option<&str>) -> String {
    let chosen = from_env
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(BASE_URL);
    chosen.trim_end_matches('/').to_owned()
}
