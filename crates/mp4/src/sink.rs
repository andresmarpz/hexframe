use std::fs::File;
use std::io::{self, Seek, Write};

pub trait Sink: Write + Seek {
    fn sync(&mut self) -> io::Result<()>;
}

impl Sink for File {
    fn sync(&mut self) -> io::Result<()> {
        self.sync_data()
    }
}
