import EventListener from "@/components/EventListener";
import { FactorioToastsApi } from "@/components/FactorioToasts";
import FactorioTooltip from "@/components/FactorioTooltip";
import KeyboardListener from "@/components/KeyboardListener";
import { SignalData, SignalPreset, validateSignalData } from "@/data";
import { formatJson } from "@/jsonUtil";
import { BASE_URL, nullify, useErrM } from "@/utils";
import { createResource, createSignal, Match, onMount, Suspense, Switch } from "solid-js";
import { createStore } from "solid-js/store";

export interface SignalFormExtractionApi {
  show(preset?: SignalPreset, args?: { kind: "edit" | "copy"; name?: string }): void;
  hide(): void;
}
export interface SignalFormExtractionProps {
  ref(api: SignalFormExtractionApi): void;
  onSubmit(key: string, v: SignalPreset): void;
  showToast: FactorioToastsApi["showToast"];
  signalPresetKeys: string[];
}
export default function SignalFormExtraction(props: SignalFormExtractionProps) {
  const DEFAULT_FORM_DATA = { name: "", description: "" };

  const [currText, setCurrText] = createSignal(""); // Current data we're editing
  const [currData, setCurrData] = createSignal<SignalData | null>(null); // Current data we're editing
  const [status, setStatus] = createSignal<("copyCommand" | "copyData" | "pasteSignals" | "metadata") | null>(null);
  const [formData, setFormData] = createStore({ ...DEFAULT_FORM_DATA });
  const [codeCommand] = createResource(() => fetch(`${BASE_URL}assets/data/generate-signals.lua`).then((v) => v.text()));

  let refs = {
    dialog: null! as HTMLDialogElement,

    dataInput: nullify<HTMLTextAreaElement>(),
    name: nullify<HTMLInputElement>(),
    description: nullify<HTMLInputElement>(),
  };

  const show: SignalFormExtractionApi["show"] = (preset, { kind, name } = { kind: "edit" }) => {
    setFormData({ ...DEFAULT_FORM_DATA }); // clear previous data

    if (preset) {
      setCurrData(preset);
      setCurrText(formatJson(JSON.stringify(preset)));
      setFormData("name", name!);
      setFormData("description", preset.description);

      if (kind === "edit") {
        setStatus("pasteSignals");
      } else if (kind === "copy") {
        setStatus("copyData");
      } else {
        throw new Error(`Invalid kind "${kind}"`);
      }
    } else {
      setCurrData(null);
      setCurrText("");
      setStatus("copyCommand"); // extract from scratch
    }

    refs.dialog.showModal();
  };
  const hide: SignalFormExtractionApi["hide"] = () => {
    // Note: we reset the state on show() instead of hide()
    setStatus(null);
    refs.dialog.close();
  };

  props.ref?.({ show, hide });

  return (
    <dialog ref={refs.dialog}>
      <KeyboardListener keys={["Escape"]} onKeyDown={() => hide()} />

      <div class="backdrop flex flex-col items-center justify-center">
        <Switch>
          <Match when={status() === "copyData"}>
            <form
              class="panel flex flex-col min-h-md h-[75vh] w-[50vw]"
              method="dialog"
              onSubmit={(e) => {
                e.preventDefault();
                setStatus(null);
                hide();
              }}
            >
              <EventListener event="copy" handler={() => props.showToast("Copied to clipboard!")} />
              <KeyboardListener keys={["Enter", "e", "E"]} onKeyDown={() => setStatus(null)} />
              <div class="font-semibold mb-5 text-tan-500 text-xl">Signal Preset JSON Data for "{formData.name}"</div>
              <div class="text-white-500">
                <div>This is the raw JSON data for the currently selected preset "{formData.name}".</div>
                <hr class="my-1" />
                {(() => {
                  const { signals: sigs, qualities: quals, electric_poles: poles } = currData()!;
                  return (
                    <div>
                      <div>
                        <span class="text-tan-500 font-semibold">Unique Signals:</span>{" "}
                        <span class="text-white font-light">
                          {sigs.length * quals?.length || 1} ({sigs.length ?? 0} base signals × {quals?.length || 1}{" "}
                          {(quals?.length || 1) === 1 ? "quality" : "qualities"})
                        </span>
                      </div>
                      <div>
                        <span class="text-tan-500 font-semibold">Qualities:</span>{" "}
                        {quals?.length ? (
                          <span class="text-white font-light">
                            {quals.length} ({quals.join(", ")})
                          </span>
                        ) : (
                          <span class="text-white font-light italic">(No qualities)</span>
                        )}
                      </div>
                      <div>
                        <span class="text-tan-500 font-semibold">Power Poles:</span>{" "}
                        {poles?.length ? (
                          <span class="text-white font-light">
                            {poles.length} ({poles.map((v) => v.name).join(", ")})
                          </span>
                        ) : (
                          <span class="text-white font-light italic">(No power poles)</span>
                        )}
                      </div>
                    </div>
                  );
                })()}
              </div>

              <div class="my-5" />

              <textarea class="w-full h-full" ref={(e) => onMount(() => (e.focus(), e.select()))} readonly value={currText()} />

              <div class="my-10" />
              <div class="flex justify-between w-full">
                <button
                  class="button bg-gray-100 px-4"
                  type="button"
                  onClick={(evt) => {
                    evt.preventDefault();
                    navigator.clipboard.writeText(currText()).then(() => props.showToast("Copied to clipboard!"));
                  }}
                >
                  Copy
                </button>
                <div />
                <button class="button button-green-right">Done</button>
              </div>
            </form>
          </Match>
          <Match when={status() === "copyCommand"}>
            <form class="panel flex flex-col min-h-md h-[75vh] w-[50vw]" method="dialog" onSubmit={() => setStatus("pasteSignals")}>
              <EventListener event="copy" handler={() => props.showToast("Copied to clipboard!")} />
              <KeyboardListener keys={["Enter", "e", "E"]} onKeyDown={() => setStatus("pasteSignals")} />
              <div class="font-semibold mb-5 text-tan-500 text-xl">Create Signal Preset</div>
              <div class="text-white-500">
                <div>
                  This tool extracts all signals and qualities available in a Factorio save file, including those provided by external mods.
                  Use this tool for compatibility with different versions of Factorio, or to utilise extra signals / qualities provided by
                  mods. This tool only needs to be run once for the specific mod combination. It can be run using any save file.
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
                <textarea
                  class="w-full h-full"
                  ref={(e) => onMount(() => (e.focus(), e.select()))}
                  readonly
                  value={`/c ${codeCommand()}`}
                />
              </Suspense>

              <div class="my-10" />
              <div class="flex justify-between w-full">
                <button
                  class="button bg-gray-100 px-4"
                  type="button"
                  onClick={(evt) => {
                    evt.preventDefault();
                    navigator.clipboard.writeText(`/c ${codeCommand()}`).then(() => props.showToast("Copied to clipboard!"));
                  }}
                >
                  Copy
                </button>
                <div />
                <button class="button button-green-right">Next</button>
              </div>
            </form>
          </Match>
          <Match when={status() === "pasteSignals"}>
            <form
              class="panel flex flex-col min-h-md h-[75vh] w-[50vw]"
              onSubmit={() => {
                const { signals, qualities } = currData()!;
                props.showToast(`Successfully imported ${signals.length} signals and ${qualities.length} qualities!`);
                setStatus("metadata");
              }}
            >
              <div class="font-semibold mb-5 text-tan-500 text-xl">Signal and Quality Extraction: Paste Signals</div>
              <div class="text-white-500">
                <div>Please paste the signals below.</div>
              </div>
              <div class="my-5" />
              <textarea
                ref={refs.dataInput!}
                class="w-full h-full"
                required
                value={currText()}
                onChange={(evt) => {
                  evt.preventDefault();

                  setCurrText(evt.currentTarget.value);

                  const errM = useErrM(evt.currentTarget);
                  let parsed = null;

                  try {
                    parsed = JSON.parse(evt.currentTarget.value);
                    if (validateSignalData(parsed)) {
                      setCurrData(parsed);
                    } else {
                      errM.report("JSON did not match expected format");
                      props.showToast("JSON did not match expected format", { isError: true });
                    }
                  } catch (e) {
                    errM.report("Invalid JSON");
                    props.showToast("Invalid JSON", { isError: true });
                  }

                  if (!errM.test()) return;

                  setFormData("description", parsed.description);
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
          </Match>
          <Match when={status() === "metadata"}>
            <form
              class="panel flex flex-col min-h-md h-[75vh] w-[50vw]"
              onSubmit={(e) => {
                e.preventDefault();
                const { name, description } = formData;
                props.onSubmit(name, { ...(currData() as SignalData), description });
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
                    <FactorioTooltip name="Preset Name" splash="Must be unique!" />
                  </label>
                  <input
                    class="bg-gray-100 focus:bg-tan-500 flex-grow px-3 py-2 border rounded focus:outline-none focus:ring"
                    ref={refs.name!}
                    type="text"
                    id="presetName"
                    name="presetName"
                    value={formData.name}
                    required
                    onInput={(e) => {
                      const errM = useErrM(e.currentTarget);
                      const { value } = e.currentTarget;
                      const MAX_LENGTH = 256;

                      if (props.signalPresetKeys.includes(value)) {
                        errM.report(`The name "${value}" is already in use.`);
                        return;
                      } else if (value.length > MAX_LENGTH) {
                        errM.report(`Name is too long (${value.length} characters, max ${MAX_LENGTH}).`);
                      }

                      errM.test() && setFormData("name", e.currentTarget.value);
                    }}
                  />
                </div>
                <div class="flex flex-row justify-between gap-5">
                  <label class="text-white-500" for="presetDescription">
                    Description
                    <FactorioTooltip name="Description" tooltip="(Optional) A description of the preset." />
                  </label>
                  <input
                    class="bg-gray-100 focus:bg-tan-500 flex-grow px-3 py-2 border rounded focus:outline-none focus:ring"
                    ref={refs.description!}
                    type="text"
                    id="presetDescription"
                    name="presetDescription"
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
          </Match>
        </Switch>
      </div>
    </dialog>
  );
}
