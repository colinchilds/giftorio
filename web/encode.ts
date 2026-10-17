export async function encodeGif(frames: { canvas: CanvasImageSource; delay: number }[], width: number, height: number) {
  const { encode } = await import("modern-gif");
  const workerUrl = (await import("modern-gif/worker?url")).default;
  const output = await encode({ workerUrl, width, height, frames: frames.map(({ canvas: data, delay }) => ({ data, delay })) });
  return new Blob([output], { type: "image/gif" });
}
