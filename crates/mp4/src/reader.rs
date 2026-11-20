use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxRef {
    pub fourcc: [u8; 4],
    pub start: usize,
    pub size: u64,
    pub payload: Range<usize>,
    pub complete: bool,
}

impl BoxRef {
    pub fn end(&self) -> usize {
        self.payload.end
    }

    pub fn children<'a>(&self, data: &'a [u8]) -> Boxes<'a> {
        Boxes::within(data, self.payload.clone())
    }

    pub fn payload_bytes<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        &data[self.payload.clone()]
    }

    pub fn find(&self, data: &[u8], fourcc: &[u8; 4]) -> Option<BoxRef> {
        self.children(data).find(|b| &b.fourcc == fourcc)
    }
}

#[derive(Debug, Clone)]
pub struct Boxes<'a> {
    data: &'a [u8],
    pos: usize,
    end: usize,
}

pub fn boxes(data: &[u8]) -> Boxes<'_> {
    Boxes::within(data, 0..data.len())
}

pub fn find_box(data: &[u8], fourcc: &[u8; 4]) -> Option<BoxRef> {
    boxes(data).find(|b| &b.fourcc == fourcc)
}

impl<'a> Boxes<'a> {
    pub fn within(data: &'a [u8], range: Range<usize>) -> Self {
        let end = range.end.min(data.len());
        Self {
            data,
            pos: range.start.min(end),
            end,
        }
    }
}

impl Iterator for Boxes<'_> {
    type Item = BoxRef;

    fn next(&mut self) -> Option<BoxRef> {
        let start = self.pos;
        let header = self.data.get(start..self.end.min(start + 8))?;
        if header.len() < 8 {
            return None;
        }
        let size32 = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let fourcc = [header[4], header[5], header[6], header[7]];
        let (size, header_len) = match size32 {
            0 => ((self.end - start) as u64, 8),
            1 => {
                let large = self.data.get(start + 8..self.end.min(start + 16))?;
                let bytes: [u8; 8] = large.try_into().ok()?;
                (u64::from_be_bytes(bytes), 16)
            }
            n => (u64::from(n), 8),
        };
        if size < header_len as u64 {
            return None;
        }
        let available = (self.end - start) as u64;
        let complete = size <= available;
        let box_end = start + usize::try_from(size.min(available)).ok()?;
        self.pos = box_end;
        if !complete {
            self.pos = self.end;
        }
        Some(BoxRef {
            fourcc,
            start,
            size,
            payload: start + header_len..box_end,
            complete,
        })
    }
}

#[cfg(test)]
mod tests;
