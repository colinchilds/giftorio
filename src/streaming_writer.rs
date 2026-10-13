use std::{
    collections::VecDeque,
    io::{self, Write},
    sync::{Arc, Mutex},
};

pub type ChunkQueue = Arc<Mutex<VecDeque<Vec<u8>>>>;

pub struct StreamingWriter {
    buffer: Vec<u8>,
    chunks: ChunkQueue,
    threshold: usize,
}

impl StreamingWriter {
    pub fn new(threshold: usize, chunks: ChunkQueue) -> Self {
        Self {
            buffer: Vec::with_capacity(threshold),
            chunks,
            threshold,
        }
    }

    fn emit_chunks(&mut self) {
        while self.buffer.len() >= self.threshold {
            let remainder = self.buffer.split_off(self.threshold);
            let chunk = std::mem::replace(&mut self.buffer, remainder);
            self.chunks.lock().unwrap().push_back(chunk);
        }
    }

    pub fn finish(&mut self) {
        if !self.buffer.is_empty() {
            self.chunks
                .lock()
                .unwrap()
                .push_back(std::mem::take(&mut self.buffer));
        }
    }
}

impl Write for StreamingWriter {
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
