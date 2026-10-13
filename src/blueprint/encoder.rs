use crate::blueprint::blueprint::BlueprintGenerator;
use crate::constants::FACTORIO_VERSION_PREFIX;
use crate::image_processing::FrameData;
use crate::models::{BlueprintArgs, OutputFormat};
use crate::progress::report_progress;
use crate::streaming_writer::{ChunkQueue, StreamingWriter};
use crate::JsValue;
use base64::{
    engine::{general_purpose::STANDARD, GeneralPurpose},
    write::EncoderWriter,
};
use flate2::{write::ZlibEncoder, Compression};
use std::io::Write;
use wasm_bindgen::UnwrapThrowExt;

pub struct BlueprintEncoder<'a> {
    args: &'a BlueprintArgs,
    blueprint_generator: BlueprintGenerator<'a>,
    chunks: ChunkQueue,
    finished: bool,
    started: bool,
    writer: Option<ZlibEncoder<OutputWriter<'a>>>,
}
enum OutputWriter<'a> {
    RawCompressed(StreamingWriter),
    Base64(EncoderWriter<'a, GeneralPurpose, StreamingWriter>),
}
impl<'a> std::io::Write for OutputWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::RawCompressed(w) => w.write(buf),
            Self::Base64(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::RawCompressed(w) => w.flush(),
            Self::Base64(w) => w.flush(),
        }
    }
}

impl<'a> BlueprintEncoder<'a> {
    pub fn new(blueprint_generator: BlueprintGenerator<'a>, args: &'a BlueprintArgs) -> Self {
        let chunks = Arc::new(Mutex::new(VecDeque::new()));
        let stream_writer = StreamingWriter::new(Arc::clone(&chunks));

        Self {
            args,
            blueprint_generator,
            chunks,
            finished: false,
            started: false,
            writer: match args.output_format {
                OutputFormat::RawCompressed => Some(ZlibEncoder::new(
                    OutputWriter::RawCompressed(stream_writer),
                    Compression::best(),
                )),
                OutputFormat::Blueprint => Some(ZlibEncoder::new(
                    OutputWriter::Base64(EncoderWriter::new(stream_writer, &STANDARD)),
                    Compression::best(),
                )),
                _ => None,
            },
        }
    }

    pub fn new_from_frame_data(
        frame_data: Option<FrameData<'a>>,
        args: &'a BlueprintArgs,
    ) -> Result<Self, JsValue> {
        let gen = BlueprintGenerator::new(frame_data, args)?;
        Ok(Self::new(gen, args))
    }

    pub fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, JsValue> {
        // Handle Factorio version prefix
        if !self.started {
            self.started = true; // Set before we return to prevent infinite loop

            if self.args.output_format == OutputFormat::Blueprint {
                return Ok(Some(FACTORIO_VERSION_PREFIX.as_bytes().to_vec()));
            }
        }

        loop {
            // Return anything already produced by the previous call first
            if let Some(chunk) = self.chunks.lock().unwrap().pop_front() {
                return Ok(Some(chunk));
            } else if self.finished {
                return Ok(None); // If we're done, return None
            }

            if !self.blueprint_generator.done() {
                if self.writer.is_some() {
                    self.blueprint_generator
                        .next_chunk(&mut self.writer.as_mut().unwrap())?;
                } else {
                    let mut buf = Vec::new();
                    self.blueprint_generator.next_chunk(&mut buf)?;
                    self.chunks.lock().unwrap().push_back(buf);
                }
                continue;
            } else if self.writer.is_none() {
                return Ok(None);
            }

            report_progress(1.0, "Finishing...");

            let zlib = self.writer.take().unwrap();
            if let OutputWriter::Base64(mut b64) = zlib.finish().unwrap_throw() {
                let _writer = b64.finish().unwrap_throw();
            }

            self.finished = true;
        }
    }
}
