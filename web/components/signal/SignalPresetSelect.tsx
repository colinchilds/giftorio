import DialogDelete, { DialogDeleteApi } from "@/components/DialogDelete";
import { ToastsApi } from "@/components/Toasts";
import Tooltip from "@/components/Tooltip";
import { CURR_SIGNAL_PRESET_KEY, SignalPreset } from "@/data";
import { addSignalPreset, DEFAULT_SIGNAL_PRESET_KEYS, getSignalPreset, getSignalPresetNames, removeSignalPreset } from "@/db";
import { createEffect, createResource, createSignal, For, onMount, Show } from "solid-js";
import { Portal } from "solid-js/web";
import SignalFormExtraction, { SignalFormExtractionApi } from "./SignalFormExtraction";

export interface SignalPresetSelectProps {
  setSignalPreset(v: SignalPreset): void;
  setSignalPresetRaw(v: Uint8Array): void;
  showToast: ToastsApi["showToast"];
}
export default function SignalPresetSelect(props: SignalPresetSelectProps) {
  const PREFIX = "signal-preset-";

  const [currPresetKey, setCurrPresetKey] = createSignal<string | null>(null);
  const [currPresetRaw, setCurrPresetRaw] = createSignal<Uint8Array | null>(null);
  const [currPreset, { mutate: setCurrPreset }] = createResource(currPresetKey, async (k) => {
    if (!k) return null;

    const { preset, raw } = await getSignalPreset(k);
    setCurrPresetRaw(raw);
    return preset;
  });
  const [signalPresetKeys, { refetch: refetchSignalPresetKeys }] = createResource(getSignalPresetNames);

  let refDialogDelete: DialogDeleteApi = null!;
  let refFormExtraction: SignalFormExtractionApi = null!;

  onMount(() => setCurrPresetKey(localStorage.getItem(CURR_SIGNAL_PRESET_KEY) ?? DEFAULT_SIGNAL_PRESET_KEYS[0]));

  createEffect(() => currPresetKey() && localStorage.setItem(CURR_SIGNAL_PRESET_KEY, currPresetKey()!));
  createEffect(() => props.setSignalPreset(currPreset()!));
  createEffect(() => props.setSignalPresetRaw(currPresetRaw()!));

  return (
    <div class="mt-5 mb-1 flex flex-col items-start gap-1 justify-between w-full">
      <DialogDelete
        ref={(api) => (refDialogDelete = api)}
        onConfirm={() => {
          removeSignalPreset(currPresetKey()!).then(() => {
            props.showToast(`Preset "${currPresetKey()}" removed!`);
            setCurrPresetKey(DEFAULT_SIGNAL_PRESET_KEYS[0]);
          });
        }}
      >
        <h2 class="text-tan-500">Delete configuration?</h2>
        <p class="text-white text-center ">This will permanently delete the configuration "{currPresetKey()}". This cannot be undone.</p>
      </DialogDelete>

      <div class="flex items-center factorio-form-element justify-between w-full">
        <label class="text-white-500" for="signalPreset">
          Signal Preset
          <Show when={currPreset()}>
            <Tooltip
              tooltip={
                <>
                  <i>{currPreset()!.description}</i>
                  <hr class="my-2" />
                  <div>
                    <div>
                      <span class="text-cyan-500 font-semibold">Total Signals:</span>{" "}
                      <span class="text-tan-500 font-semibold">{currPreset()!.signals.length}</span>{" "}
                      <span>({currPreset()!.signals.length * Math.max(currPreset()!.qualities.length, 1)} unique)</span>
                    </div>
                    <div>
                      <span class="text-cyan-500 font-semibold">Total Qualities:</span>{" "}
                      <span class="text-tan-500 font-semibold">
                        {currPreset()!.qualities.length}
                        {currPreset()!.qualities.length && <> ({currPreset()!.qualities.join(", ")})</>}
                      </span>
                    </div>
                  </div>
                </>
              }
              name={`Signal Preset: ${currPresetKey()}`}
              splash="Signal presets define the signals and qualities available for use in the blueprint."
            />
          </Show>
        </label>
        <Show
          when={currPreset()}
          fallback={
            <select>
              <option>Loading...</option>
            </select>
          }
        >
          <Portal>
            <SignalFormExtraction
              ref={(api) => (refFormExtraction = api)}
              onSubmit={(key, preset) => {
                addSignalPreset(key, preset).then(async () => {
                  await refetchSignalPresetKeys();

                  setCurrPreset(preset);
                  setCurrPresetRaw(new TextEncoder().encode(JSON.stringify(preset)));
                  setCurrPresetKey(key);

                  props.showToast(`Preset "${key}" created!`);
                  refFormExtraction.hide();
                });
              }}
              showToast={props.showToast}
              signalPresetKeys={[...(signalPresetKeys() ?? []), ...DEFAULT_SIGNAL_PRESET_KEYS]}
            />
          </Portal>
          <div class="flex gap-1">
            <div class="flex factorio-select-container">
              <select
                id="signalPreset"
                name="signalPreset"
                value={PREFIX + currPresetKey()}
                onChange={(e) => {
                  const presetKey = e.currentTarget.value.replace(PREFIX, "");
                  if (presetKey === "add-preset") {
                    refFormExtraction.show();
                    e.currentTarget.value = PREFIX + currPresetKey()!;
                  } else {
                    setCurrPresetKey(presetKey);
                  }
                }}
                title={currPreset()?.description}
              >
                <For each={DEFAULT_SIGNAL_PRESET_KEYS}>{(k) => <option value={PREFIX + k}>{k}</option>}</For>
                <For each={signalPresetKeys()}>{(k) => <option value={PREFIX + k}>{k}</option>}</For>
                <option class="italic" value="add-preset" title="Add a new preset">
                  New preset...
                </option>
              </select>
            </div>
            <button
              class="button-red w-8 h-8 text-xl font-bold"
              type="button"
              disabled={DEFAULT_SIGNAL_PRESET_KEYS.includes(currPresetKey() ?? "")}
              onClick={() => refDialogDelete.show()}
              title={
                DEFAULT_SIGNAL_PRESET_KEYS.includes(currPresetKey() ?? "")
                  ? "Cannot remove default presets"
                  : `Remove the currently selected preset "${currPresetKey()}"`
              }
            >
              🞫
            </button>
          </div>
        </Show>
      </div>
    </div>
  );
}
