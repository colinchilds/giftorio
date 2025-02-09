import init, { run_blueprint } from "./pkg/giftorio_wasm.js";
import { signals as signals_base } from "./signals.js";

async function run() {
  await init();

  addEventListener("message", async (message) => {
    const { gifData, targetFps, maxSize, useDlc, substationQuality } = message.data;
    const signals = signals_base;
    const signalsJson = JSON.stringify(signals);

    try {
      const blueprint = run_blueprint(
        gifData,
        signalsJson,
        targetFps,
        maxSize,
        substationQuality,
      );
      postMessage({ blueprint: blueprint });
    } catch (e) {
      console.error("Error generating blueprint:", e);
    }
  });
}

run();
