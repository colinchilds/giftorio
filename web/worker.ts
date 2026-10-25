import { ALL_FORMATS, BlobSource, Input, VideoSampleSink } from "mediabunny";
import init, {
  encode_gif,
  init as prepare_wasm,
  run_blueprint,
  set_comb_pos_data,
  set_progress_callback,
  set_signal_data,
} from "../pkg/giftorio_wasm.js";
import { type WasmSharedMemory } from "./types";
import { BASE_URL, nullify } from "./utils";

const FILE_PREFIX_MAP = { blueprint: "txt", json: "json", rawCompressed: "zlib" } as const;

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
    } else if (evt.data.generate) {
      const { imageData, args: blueprintArgs } = evt.data.generate;
      const { outputFormat } = blueprintArgs;
      const isVideo = blueprintArgs.imageMetadata.mimeType.startsWith("video/");

      let videoInput: Input | null = null;
      let videoSink: VideoSampleSink | null = null;
      let videoSamples: ReturnType<VideoSampleSink["samples"]> | null = null;

      if (isVideo) {
        videoInput = new Input({ source: new BlobSource(new Blob([imageData])), formats: ALL_FORMATS });

        const videoTrack = await videoInput.getPrimaryVideoTrack();
        if (!videoTrack) throw new Error("No video track found");
        if (!(await videoTrack.canDecode())) throw new Error("This browser cannot decode this video");

        videoSink = new VideoSampleSink(videoTrack, { optimizeForLatency: true });
        videoSamples = videoSink.samples();
      }

      type VideoSamples = ReturnType<VideoSampleSink["samples"]>;
      type VideoSampleNext = ReturnType<VideoSamples["next"]>;

      let videoLookahead: number | null = 1;

      const MAX_VIDEO_MEMORY = 512 * 1024 * 1024; // 512 MiB
      const MAX_LOOKAHEAD_FRAMES = 300;
      const videoSampleQueue: VideoSampleNext[] = [];

      let videoSampleTail: Promise<void> = Promise.resolve();
      let videoEnded = false;

      function prefetchVideoSamples() {
        while (videoSamples && !videoEnded && videoSampleQueue.length < (videoLookahead ?? 1)) {
          const pending = videoSampleTail.then(async () => {
            if (videoEnded) return { done: true, value: undefined } as Awaited<VideoSampleNext>;

            const result = await videoSamples!.next();
            if (result.done) {
              videoEnded = true;
            } else if (videoLookahead === null) {
              const bytesPerFrame = result.value!.allocationSize();
              videoLookahead = Math.min(MAX_LOOKAHEAD_FRAMES, Math.max(1, Math.floor(MAX_VIDEO_MEMORY / bytesPerFrame)));
            }

            return result;
          });

          // Keep calls to the iterator sequential, even if a request rejects.
          videoSampleTail = pending.then(
            () => {},
            () => {},
          );

          videoSampleQueue.push(pending);
        }
      }

      async function getNextVideoSample(): VideoSampleNext {
        prefetchVideoSamples();

        const pending = videoSampleQueue.shift();
        if (!pending) return { done: true, value: undefined } as Awaited<VideoSampleNext>;

        const result = await pending;
        prefetchVideoSamples(); // Refill the queue as soon as a frame is consumed.
        return result;
      }

      // Safe to do this now as we've already waited for init()
      if (wasmReadyPromises.setCombPosData === null) {
        wasmReadyPromises.setCombPosData = (async () => {
          const data = await fetch(`${BASE_URL}assets/data/combinator-positions.json`).then((v) => v.arrayBuffer());
          await set_comb_pos_data(new Uint8Array(data));
        })();
      }

      await wasmReadyPromises.setCombPosData;
      await wasmReadyPromises.setSignalData;

      if (isVideo) {
        prefetchVideoSamples();
      }

      try {
        waitForWriterReady();

        const prefix = FILE_PREFIX_MAP[outputFormat as keyof typeof FILE_PREFIX_MAP];
        if (!prefix) throw new Error(`Unknown output format: ${outputFormat}`);

        postMessage({ writerArgs: { type: "start", filename: `blueprint.${prefix}` } });

        await writerReady;

        const blueprintMetadata = await run_blueprint(imageData, blueprintArgs, {
          onChunk(data: Uint8Array) {
            return new Promise<void>((resolve, reject) => {
              try {
                const id = nextWriteId++;
                pendingWrites.set(id, { resolve, reject });
                postMessage({ writerArgs: { chunk: { id, data } } }, { targetOrigin: "*", transfer: [data.buffer] });
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
            const { value: sample, done } = await getNextVideoSample();

            if (done) {
              return { isFinished: true };
            }

            try {
              const [width, height] = dimensions;
              const { canvas, ctx } = state;
              if (!canvas || !ctx) throw new Error("Canvas is not initialized");

              if (canvas.width !== width || canvas.height !== height) {
                canvas.width = width;
                canvas.height = height;
              }
              const { duration, timestamp } = sample;

              // Mediabunny handles the sample's rotation and flip metadata.
              sample.draw(ctx, 0, 0, width, height);

              const pixels = ctx.getImageData(0, 0, width, height).data;
              const dest = new Uint8Array(inBuffer.memory.buffer, inBuffer.ptr, inBuffer.len);

              if (pixels.byteLength > dest.byteLength) throw new Error("Video frame exceeds WASM buffer capacity");

              dest.set(pixels);

              return { isFinished: false, duration, timestamp };
            } finally {
              sample.close();
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
        });
        postMessage({ blueprintMetadata });
        postMessage({ writerArgs: { type: "done" } });
      } catch (e) {
        postMessage({ error: e?.toString() ?? String(e) });
      }
    } else if (evt.data.signalPresetRaw) {
      const preset: Uint8Array = evt.data.signalPresetRaw;
      wasmReadyPromises.setSignalData = set_signal_data(preset);
    } else if (evt.data.type === "writerReady") {
      writerReadyResolve?.();
      writerReadyResolve = null;
    } else if (evt.data.encodeToGif) {
      const { imageData } = evt.data.encodeToGif;
      const videoInput = new Input({ source: new BlobSource(new Blob([imageData])), formats: ALL_FORMATS });
      const videoTrack = await videoInput.getPrimaryVideoTrack();
      if (!videoTrack) throw new Error("No video track found");
      if (!(await videoTrack.canDecode())) throw new Error("This browser cannot decode this video");

      const [width, height] = [await videoTrack.getDisplayWidth(), await videoTrack.getDisplayHeight()];

      const videoSink = new VideoSampleSink(videoTrack);
      const videoSamples = videoSink.samples();

      const args = {
        dim: [width, height],
        speed: 10,
      };

      postMessage({ writerArgs: { type: "start", filename: `image.gif` } });
      await encode_gif(args, {
        async getFrameData({ inBuffer }: { inBuffer: WasmSharedMemory }) {
          console.log("getFrameData", inBuffer);
          try {
            const nextSample = await videoSamples!.next();
            if (nextSample.done) return { isFinished: true };

            const sample = nextSample.value;

            try {
              const { canvas, ctx } = state;
              if (!canvas || !ctx) throw new Error("Canvas is not initialized");
              if (canvas.width !== width || canvas.height !== height) {
                canvas.width = width;
                canvas.height = height;
              }
              const { duration, timestamp } = sample;

              // Mediabunny handles the sample's rotation and flip metadata.
              sample.draw(ctx, 0, 0, width, height);

              const pixels = ctx.getImageData(0, 0, width, height).data;
              const dest = new Uint8Array(inBuffer.memory.buffer, inBuffer.ptr, inBuffer.len);

              if (pixels.byteLength > dest.byteLength) throw new Error("Video frame exceeds WASM buffer capacity");

              dest.set(pixels);

              return { isFinished: false, duration, timestamp };
            } finally {
              sample.close();
            }
          } catch (e) {
            console.error(e);
            throw e;
          }
        },
        async onChunk(data: Uint8Array) {
          console.log("onChunk", data);
          return new Promise<void>((resolve, reject) => {
            try {
              const id = nextWriteId++;
              pendingWrites.set(id, { resolve, reject });
              postMessage({ writerArgs: { chunk: { id, data } } }, { targetOrigin: "*", transfer: [data.buffer] });
            } catch (e) {
              console.error(e);
              reject(e);
            }
          });
        },
      });
      postMessage({ writerArgs: { type: "done" } });
    } else {
      throw new Error("Worker: Unhandled message", evt.data);
    }
  });
}
main();
