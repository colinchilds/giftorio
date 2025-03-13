import init, { run_blueprint, set_progress_callback } from "../pkg/giftorio_wasm.js";

async function run() {
  await init();

  set_progress_callback((percentage, status) => {
    // Post progress updates to the main thread.
    postMessage({ progress: { percentage, status } });
  });

  addEventListener("message", async (message) => {
    const { imageData, imageType, targetFps, maxSize, useDLC, substationQuality, grayscaleBits } = message.data;

    try {
      const blueprint = run_blueprint(
        imageData,
        imageType,
        useDLC,
        targetFps,
        maxSize,
        substationQuality,
        grayscaleBits,
      );
      postMessage({ blueprint: blueprint });
    } catch (e) {
      console.error("Error generating blueprint:", e);
      postMessage({ error: e.toString() });
    }
  });
}

run();
