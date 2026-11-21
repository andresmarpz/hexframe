pub(super) struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub(super) fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let out = self.bytes.get(self.pos..self.pos.checked_add(len)?)?;
        self.pos += len;
        Some(out)
    }

    pub(super) fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    pub(super) fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|b| u16::from_be_bytes([b[0], b[1]]))
    }

    pub(super) fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub(super) fn u32_if(&mut self, present: bool) -> Option<Option<u32>> {
        if present {
            self.u32().map(Some)
        } else {
            Some(None)
        }
    }

    pub(super) fn u64(&mut self) -> Option<u64> {
        let b: [u8; 8] = self.take(8)?.try_into().ok()?;
        Some(u64::from_be_bytes(b))
    }

    pub(super) fn rest(&self) -> &'a [u8] {
        &self.bytes[self.pos..]
    }
}
