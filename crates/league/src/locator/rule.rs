pub(super) const WINDOW_CLASS: &str = "RiotWindowClass";
pub(super) const IMAGE_NAME: &str = "League of Legends.exe";

pub(super) struct Candidate<'a> {
    pub class: &'a str,
    pub image_path: &'a str,
    pub pid: u32,
}

pub(super) fn is_game_window(candidate: &Candidate, own_pid: u32) -> bool {
    candidate.pid != 0
        && candidate.pid != own_pid
        && candidate.class == WINDOW_CLASS
        && image_file_name(candidate.image_path).eq_ignore_ascii_case(IMAGE_NAME)
}

fn image_file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

pub(super) fn text_from_wide(buffer: &[u16], reported_len: usize) -> String {
    let len = reported_len.min(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}
