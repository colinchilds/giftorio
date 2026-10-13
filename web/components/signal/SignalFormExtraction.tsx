import codeCommandUrl from "@/assets/data/generate-signals.lua?url";
import EventListener from "@/components/EventListener";
import KeyboardListener from "@/components/KeyboardListener";
import { ToastsApi } from "@/components/Toasts";
import Tooltip from "@/components/Tooltip";
import { SignalData, SignalPreset, validateSignalData } from "@/data";
import { useErrM } from "@/utils";
import { createResource, createSignal, onMount, Show, Suspense } from "solid-js";
import { createStore } from "solid-js/store";

export interface SignalFormExtractionApi {
  show(): void;
  hide(): void;
}
export interface SignalFormExtractionProps {
  ref(api: SignalFormExtractionApi): void;
  onSubmit(key: string, v: SignalPreset): void;
  showToast: ToastsApi["showToast"];
  signalPresetKeys: string[];
}
export default function Extraction({ showToast, ...props }: SignalFormExtractionProps) {
  type ExtractionStatus = "copyCommand" | "pasteSignals" | "metadata" | "done";

  const DEFAULT_FORM_DATA = { name: "", description: "" };

  const [currData, setCurrData] = createSignal<SignalData | null>(null); // Current data we're editing
  const [status, setStatus] = createSignal<ExtractionStatus | null>(null);
  const [formData, setFormData] = createStore(DEFAULT_FORM_DATA);
  const [codeCommand] = createResource(() => fetch(codeCommandUrl).then((v) => v.text()));
  let refDialog: HTMLDialogElement = null!;

  function show() {
    setStatus("copyCommand");
    refDialog.showModal();
  }
  function hide() {
    setStatus(null);
    refDialog.close();
  }

  props.ref?.({ show, hide });

  return (
    <dialog ref={refDialog}>
      <KeyboardListener keys={["Escape"]} onKeyDown={() => hide()} />
      <div class="backdrop flex flex-col items-center justify-center">
        <Show when={status() === "copyCommand"}>
          <form
            class="panel flex flex-col min-h-md h-[75vh] w-[50vw]"
            method="dialog"
            onSubmit={() => {
              // const { name, description } = formData;
              // const preset = { ...(currData() as SignalData), description };
              setStatus("pasteSignals");
              // props.onSubmit(name, preset);
            }}
          >
            <EventListener event="copy" handler={() => showToast("Copied to clipboard!")} />
            <KeyboardListener keys={["Enter", "e", "E"]} onKeyDown={() => setStatus("pasteSignals")} />
            <div class="font-semibold mb-5 text-tan-500 text-xl">Create Signal Preset</div>
            <div class="text-white-500">
              <div>
                This tool extracts all signals and qualities available in a Factorio save file, including those provided by external mods.
                Use this tool for compatibility with different versions of Factorio, or to utilise extra signals / qualities provided by
                mods.
              </div>
              <br />
              <div class="font-semibold">Steps:</div>
              <div>1. Create a new empty save file. Enable all mods you wish to extract signals and qualities from.</div>
              <div>2. Open the Factorio console.</div>
              <div>
                3. Copy-and-paste the command below into the Factorio console, and hit <code>Enter</code>.
              </div>
              <div>
                4. A pop-up will be displayed prompting you to copy the data. Use <code>Ctrl+C</code> to copy the list.
              </div>
              <div>5. Go back to the GIFtorio, and press "Next" to continue.</div>
            </div>
            <div class="my-5" />
            <Suspense fallback="Loading script...">
              <textarea class="w-full h-full" ref={(e) => onMount(() => (e.focus(), e.select()))} readonly value={`/c ${codeCommand()}`} />
            </Suspense>
            <div class="my-10" />
            <div class="flex justify-between w-full">
              <button
                class="button bg-gray-100 px-4"
                type="button"
                onClick={(evt) => {
                  evt.preventDefault();
                  navigator.clipboard.writeText(`/c ${codeCommand()}`).then(() => showToast("Copied to clipboard!"));
                }}
              >
                Copy
              </button>
              <div />
              <button class="button button-green-right">Next</button>
            </div>
          </form>
        </Show>
        <Show when={status() === "pasteSignals"}>
          <form
            class="panel flex flex-col min-h-md h-[75vh] w-[50vw]"
            onSubmit={() => {
              const { signals, qualities } = currData()!;
              showToast(`Successfully imported ${signals.length} signals and ${qualities.length} qualities!`);
              setStatus("metadata");
            }}
          >
            <div class="font-semibold mb-5 text-tan-500 text-xl">Signal and Quality Extraction: Paste Signals</div>
            <div class="text-white-500">
              <div>Please paste the signals below.</div>
            </div>
            <div class="my-5" />
            <textarea
              class="w-full h-full"
              required
              onChange={(evt) => {
                evt.preventDefault();

                const errM = useErrM(evt.currentTarget);
                try {
                  const parsed = JSON.parse(evt.currentTarget.value);
                  if (validateSignalData(parsed)) {
                    setCurrData(parsed);
                  } else {
                    errM.report("JSON did not match expected format");
                    showToast("JSON did not match expected format", { isError: true });
                  }
                } catch (e) {
                  errM.report("Invalid JSON");
                  showToast("Invalid JSON", { isError: true });
                }
                errM.test();
              }}
            />
            <div class="my-10" />
            <div class="flex justify-between w-full">
              <button class="button bg-gray-100 px-4" type="button" onClick={() => setStatus("copyCommand")}>
                Back
              </button>
              <div />
              <button class="button button-green-right">Next</button>
            </div>
          </form>
        </Show>
        <Show when={status() === "metadata"}>
          <form
            class="panel flex flex-col min-h-md h-[75vh] w-[50vw]"
            onSubmit={() => {
              const { name, description } = formData;
              const preset = { ...(currData() as SignalData), description };
              props.onSubmit(name, preset);
            }}
          >
            <div class="font-semibold mb-5 text-tan-500 text-xl">Signal and Quality Extraction: Configuration</div>
            <div class="text-white-500 text-sm">
              <div>Configure settings for this signal preset.</div>
            </div>
            <div class="my-5" />
            <div class="flex flex-col gap-3">
              <div class="flex flex-row justify-between gap-5">
                <label class="text-white-500" for="presetName">
                  Name
                  <Tooltip name="Preset Name" splash="Must be unique!" />
                </label>
                <input
                  class="bg-gray-100 focus:bg-tan-500 flex-grow px-3 py-2 border rounded focus:outline-none focus:ring"
                  type="text"
                  id="presetName"
                  name="presetName"
                  value={formData.name}
                  required
                  onChange={(e) => {
                    const errM = useErrM(e.currentTarget);
                    const { value } = e.currentTarget;

                    if (props.signalPresetKeys.includes(value)) {
                      console.log(value);
                      errM.report(`The name "${value}" is already in use.`);
                      return;
                    }

                    errM.test() && setFormData("name", e.currentTarget.value);
                  }}
                />
              </div>
              <div class="flex flex-row justify-between gap-5">
                <label class="text-white-500" for="presetName">
                  Description
                  <Tooltip name="Description" tooltip="(Optional) A description of the preset." />
                </label>
                <input
                  class="bg-gray-100 focus:bg-tan-500 flex-grow px-3 py-2 border rounded focus:outline-none focus:ring"
                  type="text"
                  id="presetName"
                  name="presetName"
                  value={formData.description}
                  onChange={(e) => setFormData("description", e.currentTarget.value)}
                />
              </div>
            </div>
            <div class="h-full" />
            <div class="my-10" />
            <div class="flex justify-between w-full">
              <button class="button bg-gray-100 px-4" type="button" onClick={() => setStatus("pasteSignals")}>
                Back
              </button>
              <div />
              <button class="button button-green-right">Import</button>
            </div>
          </form>
        </Show>
      </div>
    </dialog>
  );
}
