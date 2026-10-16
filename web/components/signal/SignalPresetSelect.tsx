import FactorioDialogDelete, { FactorioDialogDeleteApi } from "@/components/FactorioDialogDelete";
import { FactorioToastsApi } from "@/components/FactorioToasts";
import FactorioTooltip from "@/components/FactorioTooltip";
import { CURR_SIGNAL_PRESET_KEY, SignalPreset } from "@/data";
import { addSignalPreset, DEFAULT_SIGNAL_PRESET_KEYS, getSignalPreset, getSignalPresetNames, removeSignalPreset } from "@/db";
import { createEffect, createMemo, createResource, createSignal, For, onMount, Show } from "solid-js";
import { Portal } from "solid-js/web";
import FactorioSelect from "../FactorioSelect";
import SignalFormExtraction, { SignalFormExtractionApi } from "./SignalFormExtraction";

export interface SignalPresetSelectProps {
  setSignalPreset(v: SignalPreset): void;
  setSignalPresetRaw(v: Uint8Array): void;
  showToast: FactorioToastsApi["showToast"];
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
  const getAllSignalPresetKeys = createMemo(() => [...(signalPresetKeys() ?? []), ...DEFAULT_SIGNAL_PRESET_KEYS]);

  let refs = {
    dialogDelete: null! as FactorioDialogDeleteApi,
    formExtraction: null! as SignalFormExtractionApi,
  };

  onMount(() => setCurrPresetKey(localStorage.getItem(CURR_SIGNAL_PRESET_KEY) ?? DEFAULT_SIGNAL_PRESET_KEYS[0]));

  createEffect(() => currPresetKey() && localStorage.setItem(CURR_SIGNAL_PRESET_KEY, currPresetKey()!));
  createEffect(() => props.setSignalPreset(currPreset()!));
  createEffect(() => props.setSignalPresetRaw(currPresetRaw()!));

  return (
    <div class="mt-5 mb-1 flex flex-col items-start gap-1 justify-between w-full">
      <FactorioDialogDelete
        ref={(api) => (refs.dialogDelete = api)}
        onConfirm={() => {
          removeSignalPreset(currPresetKey()!).then(async () => {
            const deletedKey = currPresetKey()!;

            setCurrPresetKey(DEFAULT_SIGNAL_PRESET_KEYS[0]);
            await refetchSignalPresetKeys();
            props.showToast(`Preset "${deletedKey}" removed!`);
          });
        }}
      >
        <h2 class="text-tan-500">Delete configuration?</h2>
        <p class="text-white text-center ">This will permanently delete the configuration "{currPresetKey()}". This cannot be undone.</p>
      </FactorioDialogDelete>

      <div class="flex items-center factorio-form-element justify-between w-full">
        <label class="text-white-500" for="signalPreset">
          Signal Preset
          <Show when={currPreset()}>
            <FactorioTooltip
              tooltip={(() => {
                const { description, signals: sigs, qualities: quals, electric_poles: poles } = currPreset()!;
                return (
                  <>
                    <i class="tracking-wide">{description}</i>
                    <hr class="my-2" />
                    <div>
                      <div>
                        <span class="text-cyan-500 font-semibold">Total Signals:</span>{" "}
                        <span class="text-tan-500 font-semibold">{currPreset()!.signals.length}</span>{" "}
                        <span>({sigs.length * Math.max(quals.length, 1)} unique)</span>
                      </div>
                      <div>
                        <span class="text-cyan-500 font-semibold">Total Qualities:</span>{" "}
                        <span class="text-tan-500 font-semibold">
                          {quals.length} {quals.length && <> ({currPreset()!.qualities.join(", ")})</>}
                        </span>
                      </div>
                      <div>
                        <span class="text-cyan-500 font-semibold">Power Poles:</span>{" "}
                        <span class="text-tan-500 font-semibold">{poles.length}</span>
                      </div>
                    </div>
                  </>
                );
              })()}
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
              ref={(api) => (refs.formExtraction = api)}
              onSubmit={(key, preset) => {
                addSignalPreset(key, preset)
                  .then(refetchSignalPresetKeys)
                  .then(() => {
                    setCurrPreset(preset);
                    setCurrPresetRaw(new TextEncoder().encode(JSON.stringify(preset)));
                    setCurrPresetKey(key);
                    props.showToast(`Preset "${key}" created!`);
                    refs.formExtraction.hide();
                  });
              }}
              showToast={props.showToast}
              signalPresetKeys={getAllSignalPresetKeys()}
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
                    refs.formExtraction.show();
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

            <FactorioSelect
              name="Preset"
              key="signal-preset"
              customTitle={
                <button type="button" class="button icon-button w-8 h-8 [-webkit-text-stroke:1px_black]" title="More options">
                  ⋮
                </button>
              }
              options={(() => {
                const disabled = DEFAULT_SIGNAL_PRESET_KEYS.includes(currPresetKey() ?? "");
                return {
                  copy: {
                    name: "⧉ Copy signals",
                    tooltip: `Copy signal data JSON for the preset "${currPresetKey()}"`,
                  },
                  edit: {
                    name: "✎ Edit preset",
                    tooltip: disabled ? "Cannot edit built-in presets" : `Edit preset "${currPresetKey()}"`,
                    disabled,
                  },
                  delete: {
                    name: "✕ Delete preset",
                    tooltip: disabled ? "Cannot delete built-in presets" : `Delete preset "${currPresetKey()}"`,
                    disabled,
                  },
                };
              })()}
              onChange={(v) => {
                if (v === "copy") {
                  refs.formExtraction.show(currPreset()!, { kind: "copy", name: currPresetKey()! });
                } else if (v === "edit") {
                  refs.formExtraction.show(currPreset()!, { kind: "edit", name: currPresetKey()! });
                } else if (v === "delete") {
                  refs.dialogDelete.show();
                }
              }}
            />
          </div>
        </Show>
      </div>
    </div>
  );
}
