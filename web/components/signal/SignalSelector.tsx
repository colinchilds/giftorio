import FactorioSelect from "@/components/FactorioSelect";
import Tooltip from "@/components/Tooltip";
import { SignalData } from "@/data";
import { useErrM } from "@/utils";
import { createEffect, createSignal } from "solid-js";
import { TimingSignal } from "./SignalSelectors";

export interface SignalSelectorProps {
  allowedTypeMap: Map<string, string[]>;
  onChange(v: TimingSignal): void;
  otherValues: TimingSignal[];
  signalData: SignalData;
  value: TimingSignal;
}
export function SignalSelector(props: SignalSelectorProps) {
  const [currValue, setCurrValue] = createSignal(props.value);

  createEffect(() => props.onChange(currValue()));

  function hasSignal(signal: TimingSignal) {
    const { type, name, quality } = signal;
    return props.otherValues.find((v) => v.type === type && v.name === name && v.quality === quality);
  }

  return (
    <div class="flex flex-col w-full">
      <div class="factorio-form-element flex items-center justify-between gap-5">
        <label class="block text-white-500">
          <span>Name</span>
          <Tooltip name="Name" tooltip="The name of the signal." />
        </label>
        <input
          class="py-1 flex-1"
          list="signals"
          type="text"
          placeholder="<type>.<name>"
          value={`${currValue().type}.${currValue().name}`}
          onInput={(evt) => {
            evt.preventDefault();

            const el = evt.currentTarget;
            const errM = useErrM(el);
            const parts = el.value.split(".");

            if (parts.length !== 2) {
              errM.report("Invalid format, expected: <type>,<name>");
              return;
            }
            const [type, name] = parts.map((v) => v.trim());

            if (!props.allowedTypeMap.has(type)) {
              errM.report(
                `Signal type "${type}" not found.\n\nAvailable types: ` +
                  [...new Set(props.allowedTypeMap.keys().map((t) => `"${t}"`))].join(", "),
              );
            } else if (!props.allowedTypeMap.get(type)!.includes(name)) {
              errM.report(`Signal "${type}.${name}" does not exist.`);
            } else if (hasSignal({ ...currValue(), type, name })) {
              errM.report(`Duplicate signal "${type}.${name} with quality ${currValue().quality}"`);
            }
            if (!errM.test()) return;

            setCurrValue((prev) => ({ ...prev, name, type }));
          }}
        />
      </div>

      <FactorioSelect
        initialValue={currValue().quality}
        key="quality"
        name="Quality"
        onChange={(quality) => setCurrValue((prev) => ({ ...prev, quality }))}
        options={props.signalData.qualities.map((v) => [
          v,
          {
            name: v,
            tooltip: `Maps to the internal quality "${v}".`,
          },
        ])}
      />
    </div>
  );
}
