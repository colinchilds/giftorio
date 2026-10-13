import { SignalPreset } from "@/data.jsx";
import init, { init as prepare_wasm, run_blueprint, set_progress_callback, set_signal_data } from "../pkg/giftorio_wasm.js";

async function main() {
  const wasmReady = (async () => {
    await init();
    prepare_wasm();
    set_progress_callback((percentage: number, status: string) => postMessage({ progress: { percentage, status } }));
  })();

  let writerReadyResolve: (() => void) | null = null;
  let writerReady: Promise<void> = Promise.resolve();

  const waitForWriterReady = () => new Promise<void>((resolve) => (writerReadyResolve = resolve));

  const pendingWrites = new Map();
  let nextWriteId = 0;
  let setSignalDataPromise: Promise<void> = Promise.resolve();

  addEventListener("message", async (event) => {
    await wasmReady;

    if (event.data.type === "chunkWritten") {
      const pending = pendingWrites.get(event.data.id);
      if (!pending) return;

      pendingWrites.delete(event.data.id);
      event.data.error ? pending.reject(new Error(event.data.error)) : pending.resolve();
    } else if (event.data.generate) {
      const { imageData, args } = event.data.generate;
      const { outputFormat } = args;

      await setSignalDataPromise;

      try {
        waitForWriterReady();

        if (outputFormat === "blueprint") {
          postMessage({ type: "start", filename: "blueprint.txt" });
        } else if (outputFormat === "json") {
          postMessage({ type: "start", filename: "blueprint.json" });
        } else if (outputFormat === "rawCompressed") {
          postMessage({ type: "start", filename: "blueprint.zlib" });
        } else {
          console.error("Unknown output format:", outputFormat);
        }

        await writerReady;

        const blueprintMetadata = await run_blueprint(args, imageData, (data: Uint8Array) => {
          return new Promise<void>((resolve, reject) => {
            const id = nextWriteId++;
            pendingWrites.set(id, { resolve, reject });
            postMessage({ chunk: { id, data } }, { targetOrigin: "*", transfer: [data.buffer] });
          });
        });
        postMessage({ blueprintMetadata });
        postMessage({ type: "done" });
      } catch (e) {
        postMessage({ error: e?.toString() ?? String(e) });
      }
    } else if (event.data.signalPreset) {
      const preset: SignalPreset = event.data.signalPreset;
      setSignalDataPromise = set_signal_data(new TextEncoder().encode(JSON.stringify(preset)));
    } else if (event.data.type === "writerReady") {
      writerReadyResolve?.();
      writerReadyResolve = null;
    }
  });
}
main();
