#[derive(Debug, Clone, Copy)]
#[must_use]
pub(crate) struct Mark(usize);

#[derive(Debug, Default)]
pub(crate) struct BoxWriter {
    buf: Vec<u8>,
}

impl BoxWriter {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.buf.len()
    }

    pub(crate) fn into_inner(self) -> Vec<u8> {
        self.buf
    }

    pub(crate) fn begin(&mut self, fourcc: &[u8; 4]) -> Mark {
        let mark = Mark(self.buf.len());
        self.u32(0);
        self.bytes(fourcc);
        mark
    }

    pub(crate) fn end(&mut self, mark: Mark) {
        let size = u32::try_from(self.buf.len() - mark.0).unwrap_or(u32::MAX);
        self.buf[mark.0..mark.0 + 4].copy_from_slice(&size.to_be_bytes());
    }

    pub(crate) fn full_box(&mut self, fourcc: &[u8; 4], version: u8, flags: u32) -> Mark {
        let mark = self.begin(fourcc);
        self.u8(version);
        self.u24(flags);
        mark
    }

    pub(crate) fn boxed(&mut self, fourcc: &[u8; 4], body: impl FnOnce(&mut Self)) {
        let mark = self.begin(fourcc);
        body(self);
        self.end(mark);
    }

    pub(crate) fn full_boxed(
        &mut self,
        fourcc: &[u8; 4],
        version: u8,
        flags: u32,
        body: impl FnOnce(&mut Self),
    ) {
        let mark = self.full_box(fourcc, version, flags);
        body(self);
        self.end(mark);
    }

    pub(crate) fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub(crate) fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub(crate) fn u24(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_be_bytes()[1..]);
    }

    pub(crate) fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub(crate) fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub(crate) fn i32(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub(crate) fn fixed16_16(&mut self, v: f64) {
        self.i32((v * 65536.0).round() as i32);
    }

    pub(crate) fn fixed8_8(&mut self, v: f64) {
        self.u16((v * 256.0).round() as u16);
    }

    pub(crate) fn bytes(&mut self, v: &[u8]) {
        self.buf.extend_from_slice(v);
    }

    pub(crate) fn zeros(&mut self, count: usize) {
        self.buf.resize(self.buf.len() + count, 0);
    }
}
