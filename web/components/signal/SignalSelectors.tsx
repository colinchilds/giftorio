import Tooltip from "@/components/Tooltip";
import { SignalData } from "@/data";
import { createEffect, createResource, createSignal, For, Show } from "solid-js";
import { SignalSelector } from "./SignalSelector";

export const SIGNAL_SELECTOR_KEYS = Object.freeze(["f", "s", "t"] as const);
export type TimingSignal = SignalData["signals"][number] & { quality: string };
export type TimingSignals = Record<(typeof SIGNAL_SELECTOR_KEYS)[number], TimingSignal>;

export interface SignalSelectorsProps {
  values: TimingSignals;
  signalData: SignalData;
  onChange(v: TimingSignals): void;
}
export default function SignalSelectors(props: SignalSelectorsProps) {
  const [currValues, setCurrValues] = createSignal(props.values);
  const [allowedTypeMap] = createResource(
    () => props.signalData.signals,
    (sigs) => {
      const typeMap = new Map<string, string[]>();
      for (const s of sigs) {
        typeMap.set(s.type, typeMap.get(s.type) ?? []);
        typeMap.get(s.type)!.push(s.name);
      }
      return typeMap;
    },
  );

  createEffect(() => props.onChange(currValues()));

  return (
    <div class="w-full space-evenly flex flex-col gap-5">
      <Show when={allowedTypeMap()}>
        <datalist id="signals">
          <For each={Array.from(allowedTypeMap()!.entries())}>
            {([k, v]) => <For each={v}>{(v) => <option value={`${k}.${v}`} />}</For>}
          </For>
        </datalist>

        <For each={SIGNAL_SELECTOR_KEYS}>
          {(key) => (
            <div class="flex flex-col factorio-form-element justify-between w-full">
              <label class="flex items-center text-white-500" for="maxsize">
                <div class="text-tan-500 font-semibold">Signal {key.toUpperCase()}</div>
                <Tooltip
                  name={`Signal ${key.toUpperCase()}`}
                  tooltip="Used to internally time the blueprint. All timing signals must be unique."
                  splash="Reserved for timing wires; data combinators will not use these signals."
                />
              </label>
              <SignalSelector
                otherValues={(() => {
                  const { [key]: _, ...rest } = currValues();
                  return Object.values(rest);
                })()}
                onChange={(v) => setCurrValues((prev) => ({ ...prev, [key]: v }))}
                value={currValues()[key]}
                signalData={props.signalData}
                allowedTypeMap={allowedTypeMap()!}
              />
            </div>
          )}
        </For>
      </Show>
    </div>
  );
}
