import App, { AppApi } from "@/App";
import "@/index.css";
import { ErrorBoundary, Show } from "solid-js";
import { render } from "solid-js/web";
import streamSaver from "streamsaver"; // If unsupported, this will fall back to a polyfill that buffers in RAM

streamSaver.mitm = `${import.meta.env.BASE_URL}streamsaver/mitm.html`;

let refApp: AppApi;

const STREAMSAVER_MITM = new URL(streamSaver.mitm, location.href);
const SCOPE = new URL("./", STREAMSAVER_MITM.href);
const ORIGIN = location.origin.replace(/(^\w+:|^)\/\//, "");

let downloadFrame!: HTMLIFrameElement;
let downloadUrl: string | null = null;
let writer: WritableStreamDefaultWriter<Uint8Array<ArrayBufferLike>> | null = null;

const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });

// Workaround for Firefox to force streamsaver to work
const observer = new PerformanceObserver((list) => {
  for (const entry of list.getEntries() as PerformanceResourceTiming[]) {
    if (entry.responseStatus === 404 && entry.name === downloadUrl) {
      openDownloadWindow();
    }
  }
});
observer.observe({ type: "resource" });

worker.addEventListener("message", async (event) => {
  if (event.data.type === "start") {
    const { filename } = event.data;
    const pathname = `${Math.random().toString(36).slice(2)}-${Date.now().toString(36).slice(2)}/${filename}`;

    downloadUrl = new URL(`${SCOPE.href}${ORIGIN}/${pathname}`).toString();
    refApp.setDownloadUrl(downloadUrl);
    writer = streamSaver.createWriteStream(filename, { pathname }).getWriter();

    worker.postMessage({ type: "writerReady" });
  } else if (event.data.chunk) {
    const { id, data } = event.data.chunk;

    try {
      await writer!.write(data); // Safe to await now
      worker.postMessage({ type: "chunkWritten", id });
    } catch (e) {
      console.error(e);
      worker.postMessage({ type: "chunkWritten", id, error: e?.toString() ?? String(e) });
    }
  } else if (event.data.type === "done") {
    await writer!.close();
    writer = null;
  }
});

window.addEventListener("beforeunload", () => writer?.abort());

function openDownloadWindow() {
  refApp.showOpenStreamingWindowWarning();

  const a = document.createElement("a");
  a.href = downloadUrl!;
  a.target = downloadFrame.name;
  a.click();
}
const root = document.getElementById("root");

render(() => {
  return (
    <ErrorBoundary
      fallback={(err) => {
        console.error(err);
        return (
          <div class="w-screen h-screen bg-gray-800">
            <pre class="whitespace-pre-wrap text-red-300 p-4 rounded">
              <h2>Error:</h2>
              <div>{String(err)}</div>
              <Show when={err instanceof Error}>
                <hr class="my-5" />
                <h2>Stack Trace:</h2>
                <div>{(err as Error).stack}</div>
              </Show>
            </pre>
          </div>
        );
      }}
    >
      <iframe ref={downloadFrame} class="hidden" name="giftorio-download" />
      <App ref={(api) => (refApp = api)} worker={worker} streamingAvailable={streamSaver.supported} />
    </ErrorBoundary>
  );
}, root!);
