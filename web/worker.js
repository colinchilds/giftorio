import init, { run_blueprint, set_progress_callback } from "../pkg/giftorio_wasm.js";

async function run() {
  await init();

  set_progress_callback((percentage, status) => {
    // Post progress updates to the main thread.
    postMessage({ progress: { percentage, status } });
  });

  addEventListener("message", async (message) => {
    const { imageData, imageType, targetFps, maxSize, useDLC, substationQuality, paletteSize, grayscale, splitFrames, name, outputFormat } = message.data;

    try {
      // Blueprints can be hundreds of MB, so collect them as Blob parts rather than one string.
      const parts = [];
      run_blueprint(
        imageData,
        imageType,
        useDLC,
        targetFps,
        maxSize,
        substationQuality,
        paletteSize,
        grayscale,
        splitFrames,
        name,
        outputFormat === "json",
        (chunk) => parts.push(new Blob([chunk])),
      );
      postMessage({ blueprint: new Blob(parts, { type: "text/plain" }) });
    } catch (e) {
      console.error("Error generating blueprint:", e);
      postMessage({ error: e.toString() });
    }
  });
}

run();
