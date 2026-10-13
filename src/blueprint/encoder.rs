use crate::blueprint::blueprint::BlueprintGenerator;
use crate::blueprint::util::ResultJsExt;
use crate::constants::FACTORIO_VERSION_PREFIX;
use crate::models::{BlueprintArgs, OutputFormat};
use crate::progress::report_progress;
use crate::JsValue;
use base64::{
    engine::{general_purpose::STANDARD, GeneralPurpose},
    write::EncoderWriter,
};
use flate2::{write::ZlibEncoder, Compression};
use std::io::Write;

pub struct BlueprintEncoder<'a, W: Write> {
    gen: BlueprintGenerator<'a>,
    writer: Option<OutputWriter<'a, W>>,
}
enum OutputWriter<'a, W: Write> {
    Blueprint(ZlibEncoder<EncoderWriter<'a, GeneralPurpose, W>>),
    Json(W),
    RawCompressed(ZlibEncoder<W>),
}
impl<'a, W: Write> std::io::Write for OutputWriter<'a, W> {
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::RawCompressed(w) => w.flush(),
            Self::Blueprint(w) => w.flush(),
            Self::Json(w) => w.flush(),
        }
    }
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Blueprint(w) => w.write(buf),
            Self::RawCompressed(w) => w.write(buf),
            Self::Json(w) => w.write(buf),
        }
    }
}

impl<'a, W: Write> OutputWriter<'a, W> {
    pub fn finish(self) -> std::io::Result<W> {
        match self {
            Self::Blueprint(zlib) => {
                let mut b64 = zlib.finish()?;
                b64.finish()
            }
            Self::Json(mut w) => {
                w.flush()?;
                Ok(w)
            }
            Self::RawCompressed(zlib) => zlib.finish(),
        }
    }
}

impl<'a, W: Write> BlueprintEncoder<'a, W> {
    pub fn new(
        gen: BlueprintGenerator<'a>,
        args: &BlueprintArgs,
        mut stream_writer: W,
    ) -> Result<Self, JsValue> {
        if args.output_format == OutputFormat::Blueprint {
            let prefix = FACTORIO_VERSION_PREFIX.as_bytes();
            stream_writer.write_all(prefix).map_js_err("Write error")?;
        }
        let writer = Some(match args.output_format {
            OutputFormat::RawCompressed => {
                OutputWriter::RawCompressed(ZlibEncoder::new(stream_writer, Compression::best()))
            }
            OutputFormat::Blueprint => OutputWriter::Blueprint(ZlibEncoder::new(
                EncoderWriter::new(stream_writer, &STANDARD),
                Compression::best(),
            )),
            _ => OutputWriter::Json(stream_writer),
        });

        Ok(Self { gen, writer })
    }

    pub fn next(&mut self) -> Result<Option<()>, JsValue> {
        if !self.gen.done() {
            self.gen.next_chunk(&mut self.writer.as_mut().unwrap())?;
            return Ok(Some(()));
        }
        Ok(None)
    }
    pub fn finish(mut self) -> Result<W, JsValue> {
        report_progress(1.0, "Finishing...");

        self.writer
            .take()
            .unwrap()
            .finish()
            .map_js_err("Write error")
    }
}
