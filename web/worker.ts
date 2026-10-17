import init, {
  init as prepare_wasm,
  run_blueprint,
  set_comb_pos_data,
  set_progress_callback,
  set_signal_data,
} from "../pkg/giftorio_wasm.js";
import { BASE_URL } from "./utils.js";

const FILE_PREFIX_MAP = { blueprint: "txt", json: "json", rawCompressed: "zlib" } as const;

export function renderFrame(
  canvas: OffscreenCanvas,
  ctx: OffscreenCanvasRenderingContext2D,
  pixels: Uint8Array,
  width: number,
  height: number,
) {
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }
  const data = new Uint8ClampedArray(pixels.buffer as ArrayBuffer, pixels.byteOffset, pixels.byteLength);
  ctx.putImageData(new ImageData(data, width, height), 0, 0);
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

  const state = { shouldShowPreview: false };
  const waitForWriterReady = () => new Promise<void>((resolve) => (writerReadyResolve = resolve));

  const pendingWrites = new Map();
  let nextWriteId = 0;

  // Immediately add event listeners
  addEventListener("message", async (event) => {
    await wasmReadyPromises.init;

    if (event.data.type === "chunkWritten") {
      const pending = pendingWrites.get(event.data.id);
      if (!pending) return;

      pendingWrites.delete(event.data.id);
      event.data.error ? pending.reject(new Error(event.data.error)) : pending.resolve();
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
              const id = nextWriteId++;
              pendingWrites.set(id, { resolve, reject });
              postMessage({ chunk: { id, data } }, { targetOrigin: "*", transfer: [data.buffer] });
            });
          },
          onFrame(pixels: Uint8Array, width: number, height: number) {
            postMessage({ preview: { pixels, width, height } }, { targetOrigin: "*", transfer: [pixels.buffer] });
          },
          shouldShowPreview() {
            postMessage({ shouldShowPreview: { showPreview: state.shouldShowPreview } });
            return state.shouldShowPreview;
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
    }
  });
}
main();
