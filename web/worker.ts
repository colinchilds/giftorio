import init, {
  init as prepare_wasm,
  run_blueprint,
  set_comb_pos_data,
  set_progress_callback,
  set_signal_data,
} from "../pkg/giftorio_wasm.js";
import { type WasmSharedMemory } from "./types";
import { BASE_URL, nullify } from "./utils";

const FILE_PREFIX_MAP = { blueprint: "txt", json: "json", rawCompressed: "zlib" } as const;
type VideoFrameMessage = {
  bitmap?: ImageBitmap;
  dimensions?: [number, number];
  isFinished: boolean;
};

export function seekTo(video: HTMLVideoElement, time: number): Promise<void> {
  if (video.readyState < HTMLMediaElement.HAVE_METADATA) {
    return new Promise((resolve, reject) => {
      video.addEventListener(
        "loadedmetadata",

        () => seekTo(video, time).then(resolve, reject),
        { once: true },
      );
    });
  } else if (Math.abs(video.currentTime - time) < 0.001) {
    return Promise.resolve();
  }

  return new Promise((resolve, reject) => {
    function onSeeked() {
      cleanup();
      resolve();
    }
    function onError() {
      cleanup();
      reject(video.error ?? new Error("Video seek failed"));
    }
    function cleanup() {
      video.removeEventListener("seeked", onSeeked);
      video.removeEventListener("error", onError);
    }

    // Register first, so a fast seek cannot beat the listener.
    video.addEventListener("seeked", onSeeked, { once: true });
    video.addEventListener("error", onError, { once: true });

    video.currentTime = time;
  });
}

async function main() {
  const wasmReadyPromises = {
    init: (async () => {
      await init();
      prepare_wasm();
      set_progress_callback((percentage: number, status: string) => postMessage({ progress: { percentage, status } }));
    })(),
    setCombPosData: null as Promise<void> | null,
    setSignalData: Promise.resolve(),
  };

  // For preview of the rendered GIF
  let writerReadyResolve: (() => void) | null = null;
  let writerReady: Promise<void> = Promise.resolve();

  const state = {
    canvas: nullify<OffscreenCanvas>(),
    ctx: nullify<OffscreenCanvasRenderingContext2D>(),

    shouldShowPreview: false,
  };
  const waitForWriterReady = () => new Promise<void>((resolve) => (writerReadyResolve = resolve));

  const pendingWrites = new Map();
  let nextWriteId = 0;

  // Used to allow rust to yield to JS at regular intervals
  const syncChannel = new MessageChannel();
  const syncQ: (() => void)[] = [];

  syncChannel.port1.onmessage = () => syncQ.shift()?.();

  const videoFrameQueue: VideoFrameMessage[] = [];
  const videoFrameWaiters: Array<(frame: VideoFrameMessage) => void> = [];

  let videoStreamFinished = false;

  function enqueueVideoFrame(frame: VideoFrameMessage) {
    if (frame.isFinished) {
      videoStreamFinished = true;
    }
    const waiter = videoFrameWaiters.shift();
    if (waiter) {
      waiter(frame);
    } else {
      videoFrameQueue.push(frame);
    }
  }

  function nextVideoFrame(): Promise<VideoFrameMessage> {
    const frame = videoFrameQueue.shift();
    if (frame) {
      return Promise.resolve(frame);
    } else if (videoStreamFinished) {
      return Promise.resolve({ isFinished: true });
    } else {
      return new Promise((resolve) => videoFrameWaiters.push(resolve));
    }
  }

  // Immediately add event listeners
  addEventListener("message", async (evt) => {
    await wasmReadyPromises.init;

    if (evt.data.type === "chunkWritten") {
      const pending = pendingWrites.get(evt.data.id);
      if (!pending) return;

      pendingWrites.delete(evt.data.id);
      evt.data.error ? pending.reject(new Error(evt.data.error)) : pending.resolve();
    } else if (evt.data.canvas) {
      const { canvas } = evt.data.canvas;
      state.canvas = canvas;
      state.ctx = state.canvas!.getContext("2d", {
        colorSpace: "srgb",
        willReadFrequently: true,
      })!;
      // state.ctx!.imageSmoothingEnabled = false;
    } else if (evt.data.showPreview) {
      const { showPreview } = evt.data.showPreview;
      state.shouldShowPreview = showPreview;
    } else if (evt.data.videoRenderFrame) {
      enqueueVideoFrame(evt.data.videoRenderFrame);
    } else if (evt.data.generate) {
      const { imageData, args: blueprintArgs } = evt.data.generate;
      const { outputFormat } = blueprintArgs;

      // Safe to do this now as we've already waited for init()
      if (wasmReadyPromises.setCombPosData === null) {
        wasmReadyPromises.setCombPosData = (async () => {
          const data = await fetch(`${BASE_URL}assets/data/combinator-positions.json`).then((v) => v.arrayBuffer());
          await set_comb_pos_data(new Uint8Array(data));
        })();
      }

      await wasmReadyPromises.setCombPosData;
      await wasmReadyPromises.setSignalData;

      try {
        waitForWriterReady();

        const prefix = FILE_PREFIX_MAP[outputFormat as keyof typeof FILE_PREFIX_MAP];
        if (!prefix) throw new Error(`Unknown output format: ${outputFormat}`);

        postMessage({ type: "start", filename: `blueprint.${prefix}` });

        await writerReady;

        const callbacks = {
          onChunk(data: Uint8Array) {
            return new Promise<void>((resolve, reject) => {
              try {
                const id = nextWriteId++;
                pendingWrites.set(id, { resolve, reject });
                postMessage({ chunk: { id, data } }, { targetOrigin: "*", transfer: [data.buffer] });
              } catch (e) {
                console.error(e);
                reject(e);
              }
            });
          },
          onFrame({ outBuffer, dimensions }: { outBuffer: WasmSharedMemory; dimensions: [number, number] }) {
            try {
              const { canvas, ctx } = state;
              const [width, height] = dimensions;

              if (!canvas || !ctx) return;
              if (canvas.width !== width || canvas.height !== height) {
                canvas.width = width;
                canvas.height = height;
              }

              const data = new Uint8ClampedArray(outBuffer.memory.buffer, outBuffer.ptr, outBuffer.len);
              ctx.putImageData(new ImageData(data, width, height), 0, 0);
            } catch (e) {
              console.error(e);
            }
          },
          async getFrameData({ inBuffer, dimensions }: { inBuffer: WasmSharedMemory; dimensions: [number, number] }) {
            try {
              // Start production when the consumer first needs a frame.
              postMessage({ videoRenderRequest: { status: "need-frame", dimensions } });

              const frame = await nextVideoFrame();
              if (frame.isFinished) {
                return { isFinished: true };
              }

              const { bitmap, dimensions: frameDimensions } = frame;
              const [width, height] = frameDimensions!;
              if (!bitmap || width !== dimensions[0] || height !== dimensions[1]) {
                bitmap?.close();
                throw new Error("Unexpected video frame dimensions");
              }

              try {
                const { canvas, ctx } = state;
                if (!canvas || !ctx) {
                  throw new Error("Canvas is not initialized");
                }

                if (canvas.width !== width || canvas.height !== height) {
                  canvas.width = width;
                  canvas.height = height;
                }

                // Reuse the SAME canvas used for preview rendering.
                ctx.drawImage(bitmap, 0, 0, width, height);

                const pixels = ctx.getImageData(0, 0, width, height).data;
                const dest = new Uint8Array(inBuffer.memory.buffer, inBuffer.ptr, inBuffer.len);

                if (pixels.byteLength > dest.byteLength) {
                  throw new Error("Video frame exceeds WASM buffer capacity");
                }

                dest.set(pixels);
              } finally {
                bitmap.close();
              }

              postMessage({ videoRenderRequest: { status: "frame-consumed" } });

              return { isFinished: false };
            } catch (e) {
              console.error(e);
              throw e;
            }
          },
          shouldShowPreview() {
            try {
              postMessage({ shouldShowPreview: { showPreview: state.shouldShowPreview } });
              return state.shouldShowPreview;
            } catch (e) {
              console.error(e);
            }
          },
          yieldToJs() {
            return new Promise<void>((resolve: () => void) => {
              syncQ.push(resolve);
              syncChannel.port2.postMessage(null);
            });
          },
        };

        const blueprintMetadata = await run_blueprint(blueprintArgs, imageData, callbacks);
        postMessage({ blueprintMetadata });
        postMessage({ type: "done" });
      } catch (e) {
        postMessage({ error: e?.toString() ?? String(e) });
      }
    } else if (evt.data.signalPresetRaw) {
      const preset: Uint8Array = evt.data.signalPresetRaw;
      wasmReadyPromises.setSignalData = set_signal_data(preset);
    } else if (evt.data.type === "writerReady") {
      writerReadyResolve?.();
      writerReadyResolve = null;
    } else {
      throw new Error("Worker: Unhandled message", evt.data);
    }
  });
}
main();
