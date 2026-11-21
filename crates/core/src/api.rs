pub const BASE_URL: &str = "https://api.greyscreen.app";
pub const BASE_URL_ENV: &str = "GREYSCREEN_API_URL";

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_env_override_wins_and_loses_its_trailing_slash() {
        assert_eq!(
            resolve_base_url(Some("http://localhost:8787/")),
            "http://localhost:8787"
        );
    }

    #[test]
    fn a_missing_or_blank_override_falls_back_to_the_constant() {
        assert_eq!(resolve_base_url(None), BASE_URL);
        assert_eq!(resolve_base_url(Some("  ")), BASE_URL);
    }
}
