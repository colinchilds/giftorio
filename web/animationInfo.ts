/**
 * Utility function to obtain the raw image data from a static image.
 *
 * This gives higher compatibility with different images than using Rust.
 * Only used for static images; videos are processed in Rust.
 */
export async function getRawImageData(file: File): Promise<ImageData> {
  const bitmap = await createImageBitmap(file);
  const ctx = new OffscreenCanvas(bitmap.width, bitmap.height).getContext("2d")!;
  ctx.drawImage(bitmap, 0, 0);

  const imageData = ctx.getImageData(0, 0, bitmap.width, bitmap.height, { colorSpace: "srgb", pixelFormat: "rgba-unorm8" });
  bitmap.close();
  return imageData;
}

export class AnimationInfo {
  /** Number of frames in the animation. */
  frames: number;

  /** Duration of the animation in milliseconds. */
  duration: number;

  /** Width of the animation in pixels. */
  width: number;

  /** Height of the animation in pixels. */
  height: number;

  constructor(frames: number, duration: number, width: number, height: number) {
    this.frames = frames;
    this.duration = duration; // milliseconds
    this.width = width;
    this.height = height;
  }
}
function gifInfo(data: Uint8Array): AnimationInfo {
  if (data.length < 13) throw new Error("GIF data is too short");

  const header = new TextDecoder().decode(data.subarray(0, 6));
  if (header !== "GIF87a" && header !== "GIF89a") throw new Error("Invalid GIF header");

  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const [width, height] = [view.getUint16(6, true), view.getUint16(8, true)];

  let pos = 13;
  const packed = data[10];

  // Global Color Table
  if (packed & 0x80) {
    const size = 3 << ((packed & 0x07) + 1);
    if (data.length - pos < size) throw new Error("Truncated global color table");
    pos += size;
  }

  let frames = 0;
  let durationCs = 0;
  let pendingDelayCs = 0; // Delay belonging to the next image descriptor.

  function skipSubBlocks() {
    while (true) {
      if (pos >= data.length) throw new Error("Truncated GIF sub-block");

      const size = data[pos++];
      if (size === 0) return;
      if (data.length - pos < size) throw new Error("Truncated GIF sub-block data");

      pos += size;
    }
  }

  while (true) {
    if (pos >= data.length) throw new Error("Missing GIF trailer");

    const block = data[pos++];
    switch (block) {
      // Image Descriptor = one frame
      case 0x2c: {
        frames++;
        if (data.length - pos < 9) throw new Error("Truncated image descriptor");
        const packed = data[pos + 8];
        pos += 9;

        // Local Color Table
        if (packed & 0x80) {
          const size = 3 << ((packed & 0x07) + 1);
          if (data.length - pos < size) throw new Error("Truncated local color table");
          pos += size;
        }
        if (pos >= data.length) throw new Error("Missing LZW code size"); // LZW minimum code size

        pos++;
        skipSubBlocks(); // LZW data sub-blocks
        durationCs += pendingDelayCs;
        pendingDelayCs = 0;
        break;
      }
      // Extension block
      case 0x21: {
        if (pos >= data.length) throw new Error("Truncated extension");

        const label = data[pos++];
        if (label === 0xf9) {
          // Graphic Control Extension:
          // 21 F9 04 [packed] [delay lo] [delay hi]
          //    [transparent] 00
          if (data.length - pos < 6) throw new Error("Truncated graphic control extension");

          const blockSize = data[pos];
          if (blockSize !== 4) throw new Error("Invalid graphic control extension");

          pendingDelayCs = data[pos + 2] | (data[pos + 3] << 8); // Delay is little-endian, in centiseconds.
          pos += 6;
        } else skipSubBlocks(); // Other extension data
        break;
      }
      case 0x3b:
        return new AnimationInfo(frames, durationCs * 10, width, height);
      default:
        throw new Error("Invalid GIF block introducer");
    }
  }
}

function webpInfo(data: Uint8Array): AnimationInfo {
  if (data.length < 12) throw new Error("WebP data is too short");

  const header = new TextDecoder().decode(data.subarray(0, 4));
  const format = new TextDecoder().decode(data.subarray(8, 12));

  if (header !== "RIFF" || format !== "WEBP") throw new Error("Invalid WebP header");

  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const riffSize = view.getUint32(4, true);
  const end = 8 + riffSize;

  if (end > data.length) throw new Error("Truncated WebP");

  let pos = 12;
  let frames = 0;
  let durationMs = 0;

  // Read image dimensions
  let width, height;
  while (end - pos >= 8) {
    const chunkType = String.fromCharCode(data[pos], data[pos + 1], data[pos + 2], data[pos + 3]);
    const chunkSize = view.getUint32(pos + 4, true);
    pos += 8;

    if (chunkSize > end - pos) throw new Error("Truncated WebP chunk");
    if (chunkType === "VP8X") {
      if (chunkSize < 10) throw new Error("Invalid VP8X chunk");
      width = 1 + (data[pos + 4] | (data[pos + 5] << 8) | (data[pos + 6] << 16));
      height = 1 + (data[pos + 7] | (data[pos + 8] << 8) | (data[pos + 9] << 16));
    } else if (chunkType === "ANMF") {
      if (chunkSize < 16) throw new Error("Invalid ANMF chunk");
      frames++;
      // ANMF:
      //   bytes 0..3   X
      //   bytes 3..6   Y
      //   bytes 6..9   width
      //   bytes 9..12  height
      //   bytes 12..15 duration (24-bit little endian)
      const frameDurationMs = data[pos + 12] | (data[pos + 13] << 8) | (data[pos + 14] << 16);
      durationMs += frameDurationMs;
    }
    pos += chunkSize;
    if (chunkSize & 1) pos++; // RIFF chunks are padded to an even size.
  }
  if (width === undefined || height === undefined) throw new Error("Invalid WebP dimensions");
  return new AnimationInfo(frames, durationMs, width, height);
}

/**
 * Retrieves information about the animation in the given image data.
 *
 * @param data - Image data to analyze.
 * @returns Information about the animation.
 */
export default function animationInfo(data: Uint8Array): AnimationInfo {
  if (data.length >= 6 && ["GIF87a", "GIF89a"].includes(String.fromCharCode(...data.subarray(0, 6)))) {
    return gifInfo(data);
  } else if (
    data.length >= 12 &&
    String.fromCharCode(...data.subarray(0, 4)) === "RIFF" &&
    String.fromCharCode(...data.subarray(8, 12)) === "WEBP"
  ) {
    return webpInfo(data);
  } else throw new UnsupportedImageError("Unsupported image format");
}

export class UnsupportedImageError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "UnsupportedImageError";
  }
}
