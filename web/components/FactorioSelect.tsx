import Tooltip from "@/components/Tooltip";
import { FormElementValue } from "@/types";
import { createEffect, createSignal, For, JSX, Show } from "solid-js";

export type SelectOption = { name: string; tooltip?: string };
export interface FactorioSelectProps extends Omit<JSX.SelectHTMLAttributes<HTMLSelectElement>, "onChange"> {
  formArgs?: FormElementValue;
  initialValue: string;
  key: string;
  name?: string;
  onChange(value: string): void;
  tooltip?: string;
  options: (readonly [string, string | SelectOption])[];
  splash?: string;
}
export default function FactorioSelect(props: FactorioSelectProps) {
  let selectProps: JSX.SelectHTMLAttributes<HTMLSelectElement>;
  {
    const { formArgs, key, name, disabled, onChange, options, tooltip, splash, ref, ..._selectProps } = props;
    selectProps = _selectProps;
  }

  const currValue = () => props.initialValue;
  const [selectTitle, setSelectTitle] = createSignal<string>();

  function handleChange(e: Event) {
    props.onChange((e.currentTarget as HTMLSelectElement).value);
  }

  createEffect(() => {
    const value = currValue();
    const option = props.options.find(([k]) => k === value);
    if (!option) return;

    const [, info] = option;

    if (typeof info === "string") {
      setSelectTitle(props.tooltip);
    } else {
      setSelectTitle(`${props.tooltip}\n\n${info.name}: ${info.tooltip}`);
    }
  });

  return (
    <div
      class="mt-1 mb-1 flex items-center justify-between factorio-form-element factorio-select-container"
      aria-disabled={props.formArgs?.disabled || props.disabled}
    >
      <Show when={props.name}>
        <label class="block text-white-500" for={props.key}>
          <span classList={{ "opacity-50": props.disabled }}>{props.name}</span>
          <Tooltip name={props.name!} tooltip={props.tooltip} splash={props.splash} options={props.options} />
        </label>
      </Show>
      <select
        {...selectProps}
        id={props.key}
        name={props.key}
        onChange={handleChange}
        disabled={props.disabled}
        title={selectTitle()}
        value={currValue()}
      >
        <For each={props.options}>
          {([k, v]) => (
            <option title={typeof v === "string" ? undefined : v.tooltip} value={k} selected={k === currValue()}>
              {typeof v === "string" ? v : v.name}
            </option>
          )}
        </For>
      </select>
    </div>
  );
}
