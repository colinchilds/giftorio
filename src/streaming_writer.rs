use std::io::{self, Write};

pub struct StreamingWriter<F>
where
    F: FnMut(Vec<u8>),
{
    buffer: Vec<u8>,
    threshold: usize,
    on_chunk: F,
}

impl<F> StreamingWriter<F>
where
    F: FnMut(Vec<u8>),
{
    pub fn new(threshold: usize, on_chunk: F) -> Self {
        Self {
            buffer: Vec::with_capacity(threshold),
            threshold,
            on_chunk,
        }
    }

    fn emit_chunks(&mut self) {
        while self.buffer.len() >= self.threshold {
            let remainder = self.buffer.split_off(self.threshold);
            let chunk = std::mem::replace(&mut self.buffer, remainder);
            (self.on_chunk)(chunk);
        }
    }

    pub fn finish(&mut self) {
        if !self.buffer.is_empty() {
            (self.on_chunk)(std::mem::take(&mut self.buffer));
        }
    }
}

impl<F> Write for StreamingWriter<F>
where
    F: FnMut(Vec<u8>),
{
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(data);
        self.emit_chunks();
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.emit_chunks();
        Ok(())
    }
}
