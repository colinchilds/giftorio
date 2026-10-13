import App, { AppApi } from "@/App";
import "@/index.css";
import { ErrorBoundary } from "solid-js";
import { render } from "solid-js/web";
import streamSaver from "streamsaver"; // If unsupported, this will fall back to a polyfill that buffers in RAM

streamSaver.mitm = "/streamsaver/mitm.html";

let refApp: AppApi;

const STREAMSAVER_MITM = new URL(streamSaver.mitm, location.href);
const SCOPE = new URL("./", STREAMSAVER_MITM.href);
const ORIGIN = location.origin.replace(/(^\w+:|^)\/\//, "");

let downloadUrl: string | null = null;
let prevWritePromises: Promise<void>[] = [];
let unblockWriterPromise: Promise<void> | null = null;
let writer: WritableStreamDefaultWriter<Uint8Array<ArrayBufferLike>> | null = null;

const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });

/**
 * On Firefox this opens a new tab with minimal size and immediately closes it.
 *
 * The tab makes a user-defined GET request to the download URL which unblocks the writer.
 * Note that making a regular GET request to the URL results in Vite redirecting to index.html
 */
async function unblockWriter() {
  // Test 10 writes of empty data. The writer internally buffers writes to reduce backpressure,
  // so we are unable to detect if it is blocked from testing a singular write. Typically this will
  // block on the 2nd or 3rd writes. If 250ms pass without completing the test we open a new tab.
  const writes = Array.from({ length: 10 }, () => writer!.write(new Uint8Array()));
  const timeout = new Promise((_, reject) => setTimeout(reject, 250));

  unblockWriterPromise = Promise.race([Promise.all(writes), timeout])
    .catch(() => openDownloadWindow())
    .finally(() => (unblockWriterPromise = null)) as Promise<void>;
}

worker.addEventListener("message", async (event) => {
  if (event.data.type === "start") {
    const { filename } = event.data;
    const pathname = [Math.random(), Date.now(), filename].join("/");

    downloadUrl = new URL(`${SCOPE.href}${ORIGIN}/${pathname}`).toString();
    writer = streamSaver.createWriteStream(filename, { pathname }).getWriter();

    unblockWriter(); // Workaround for Firefox

    worker.postMessage({ type: "writerReady" });
  } else if (event.data.chunk) {
    const { id, data } = event.data.chunk;
    try {
      if (unblockWriterPromise) {
        // If we're waiting for writer unblock we don't await the write
        prevWritePromises.push(writer!.write(data));
      } else {
        // If we finished unblocking await all cached writes
        if (prevWritePromises.length) {
          await Promise.all(prevWritePromises);
          prevWritePromises.length = 0; // Clear cache
        }
        await writer!.write(data); // Safe to await now
      }
      worker.postMessage({ type: "chunkWritten", id });
    } catch (e) {
      console.error(e);
      worker.postMessage({ type: "chunkWritten", id, error: e?.toString() ?? String(e) });
    }
  } else if (event.data.type === "done") {
    try {
      await Promise.race([unblockWriterPromise, new Promise((_, reject) => setTimeout(reject, 1000))])
        .catch(() => console.log("Failed to unblock writer"))
        .finally(async () => await writer!.close());
    } finally {
      writer = null; // Clear writer
      unblockWriterPromise = null; // Clear promise
    }
  }
});

window.addEventListener("beforeunload", () => writer?.abort());

function openDownloadWindow() {
  refApp.showOpenStreamingWindowWarning();
  return window.open(downloadUrl!, "Giftorio: Streaming Download Unblocker", "noopener,noreferrer,width=100,height=100");
}
const root = document.getElementById("root");

render(() => {
  return (
    <ErrorBoundary fallback={(err) => <pre style={{ "white-space": "pre-wrap" }}>{err instanceof Error ? err.stack : String(err)}</pre>}>
      <App ref={(api) => (refApp = api)} worker={worker} streamingAvailable={streamSaver.supported} />
    </ErrorBoundary>
  );
}, root!);
