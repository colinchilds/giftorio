use std::io::{self, Write};
use wasm_bindgen::prelude::*;

pub struct StreamingWriter<F: FnMut(Vec<u8>) -> Result<(), JsValue>> {
    buffer: Vec<u8>,
    threshold: usize,
    on_chunk: F,
}

impl<F: FnMut(Vec<u8>) -> Result<(), JsValue>> StreamingWriter<F> {
    pub fn new(threshold: usize, on_chunk: F) -> Self {
        Self {
            buffer: Vec::with_capacity(threshold),
            threshold,
            on_chunk,
        }
    }

    #[inline(always)]
    fn call_on_chunk(&mut self, chunk: Vec<u8>) -> io::Result<()> {
        let result = (self.on_chunk)(chunk);
        result.map_err(|e| io::Error::other(e.as_string().unwrap_or_else(|| format!("{e:?}"))))
    }

    fn emit_chunks(&mut self) -> io::Result<()> {
        while self.buffer.len() >= self.threshold {
            let remainder = self.buffer.split_off(self.threshold);
            let chunk = std::mem::replace(&mut self.buffer, remainder);
            self.call_on_chunk(chunk)?;
        }
        Ok(())
    }

    pub fn finish(&mut self) -> io::Result<()> {
        if !self.buffer.is_empty() {
            let chunk = std::mem::take(&mut self.buffer);
            self.call_on_chunk(chunk)?;
        }
        Ok(())
    }
}

impl<F: FnMut(Vec<u8>) -> Result<(), JsValue>> Write for StreamingWriter<F> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(data);
        self.emit_chunks()?;
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.emit_chunks()?;
        Ok(())
    }
}
