import init, {
  init as prepare_wasm,
  run_blueprint,
  set_comb_pos_data,
  set_progress_callback,
  set_signal_data,
} from "../pkg/giftorio_wasm.js";
import { BASE_URL, nullify } from "./utils.js";

const FILE_PREFIX_MAP = { blueprint: "txt", json: "json", rawCompressed: "zlib" } as const;

export function getVideoPixelData(
  video: HTMLVideoElement,
  canvas: HTMLCanvasElement,
  width: number,
  height: number,
): ImageDataArray | null {
  const ctx = canvas.getContext("2d")!;

  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }

  ctx.drawImage(video, 0, 0, width, height);
  return ctx.getImageData(0, 0, width, height).data;
}

export function seekTo(video: HTMLVideoElement, time: number): Promise<void> {
  if (video.readyState < HTMLMediaElement.HAVE_METADATA) {
    return new Promise((resolve, reject) => {
      video.addEventListener(
        "loadedmetadata",
        //
        () => seekTo(video, time).then(resolve, reject),
        { once: true },
      );
    });
  }

  if (Math.abs(video.currentTime - time) < 0.001) {
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

  const channel = new MessageChannel();
  const queue: (() => void)[] = [];

  channel.port1.onmessage = () => queue.shift()?.();

  // Immediately add event listeners
  addEventListener("message", async (event) => {
    await wasmReadyPromises.init;

    if (event.data.type === "chunkWritten") {
      const pending = pendingWrites.get(event.data.id);
      if (!pending) return;

      pendingWrites.delete(event.data.id);
      event.data.error ? pending.reject(new Error(event.data.error)) : pending.resolve();
    } else if (event.data.canvas) {
      const { canvas } = event.data.canvas;
      state.canvas = canvas;
      state.ctx = canvas.getContext("2d")!;
      state.ctx!.imageSmoothingEnabled = false;
    } else if (event.data.showPreview) {
      const { showPreview } = event.data.showPreview;
      state.shouldShowPreview = showPreview;
    } else if (event.data.generate) {
      const { imageData, args } = event.data.generate;
      const { outputFormat } = args;

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
          onFrame(memory: WebAssembly.Memory, ptr: number, len: number, width: number, height: number) {
            try {
              const { canvas, ctx } = state;

              if (!canvas || !ctx) return;
              if (canvas.width !== width || canvas.height !== height) {
                canvas.width = width;
                canvas.height = height;
              }

              const data = new Uint8ClampedArray(memory.buffer, ptr, len);
              ctx.putImageData(new ImageData(data, width, height), 0, 0);
            } catch (e) {
              console.error(e);
            }
          },
          getFrameData(timestampMs: number, memory: WebAssembly.Memory, ptr: number, len: number, width: number, height: number) {
            try {
              console.log("getFrameData", timestampMs);
              return { isFinished: true };
            } catch (e) {
              console.error(e);
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
              queue.push(resolve);
              channel.port2.postMessage(null);
            });
          },
        };

        const blueprintMetadata = await run_blueprint(args, imageData, callbacks);
        postMessage({ blueprintMetadata });
        postMessage({ type: "done" });
      } catch (e) {
        postMessage({ error: e?.toString() ?? String(e) });
      }
    } else if (event.data.signalPresetRaw) {
      const preset: Uint8Array = event.data.signalPresetRaw;
      wasmReadyPromises.setSignalData = set_signal_data(preset);
    } else if (event.data.type === "writerReady") {
      writerReadyResolve?.();
      writerReadyResolve = null;
    } else if (event.data.videoRenderFrame) {
      const { pixels, width, height } = event.data.videoRenderFrame;
      const data = new Uint8ClampedArray(pixels);
      const imageData = new ImageData(data, width, height);
      console.log("videoRenderFrame", imageData);
    } else {
      throw new Error("Worker: Unhandled message", event.data);
    }
  });
}
main();
