import init, { run_blueprint, set_progress_callback } from "../pkg/giftorio_wasm.js";
import { signals as signals_base } from "./signals.js";
import { signals_dlc } from "./signals-dlc.js";

async function run() {
  await init();

  set_progress_callback((percentage, status) => {
    // Post progress updates to the main thread.
    postMessage({ progress: { percentage, status } });
  });

  addEventListener("message", async (message) => {
    const { gifData, targetFps, maxSize, useDLC, substationQuality, grayscaleBits } = message.data;
    const signals = useDLC ? signals_dlc : signals_base;
    const signalsJson = JSON.stringify(signals);

    try {
      const blueprint = run_blueprint(
        gifData,
        useDLC,
        signalsJson,
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
