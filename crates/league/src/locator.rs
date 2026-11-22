#[cfg(any(windows, test))]
mod rule;
#[cfg(windows)]
mod win;

#[cfg(test)]
mod tests;

pub trait GameLocator: Send {
    fn find(&mut self) -> Option<(isize, u32)>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct AlwaysPresent;

impl GameLocator for AlwaysPresent {
    fn find(&mut self) -> Option<(isize, u32)> {
        Some((0, 0))
    }
}

#[cfg(windows)]
pub use win::WindowLocator;
