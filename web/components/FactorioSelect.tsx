import FactorioTooltip from "@/components/FactorioTooltip";
import { FormElementValue } from "@/types";
import { maybeToEntries } from "@/utils";
import { createEffect, createSignal, For, JSX, Show } from "solid-js";

export type SelectOption = { name: string; disabled?: boolean; tooltip?: string };
export interface FactorioSelectProps extends Omit<JSX.SelectHTMLAttributes<HTMLSelectElement>, "onChange"> {
  customTitle?: JSX.Element;
  formArgs?: FormElementValue;
  initialValue?: string;
  key: string;
  name?: string;
  onChange(value: string): void;
  tooltip?: string;
  options: (readonly [string, string | SelectOption])[] | Record<string, string | SelectOption>;
  splash?: string;
}
export default function FactorioSelect(props: FactorioSelectProps) {
  let selectProps: JSX.SelectHTMLAttributes<HTMLSelectElement>;
  let select!: HTMLSelectElement;

  {
    const { formArgs, key, name, disabled, onChange, options, tooltip, splash, ref, ..._selectProps } = props;
    selectProps = _selectProps;
  }

  const currValue = () => (props.customTitle ? null : props.initialValue);
  const [selectTitle, setSelectTitle] = createSignal<string>();

  function handleChange(e: Event & { currentTarget: HTMLSelectElement }) {
    props.onChange(e.currentTarget.value);

    if (props.customTitle) {
      e.currentTarget.value = ""; // Reset to allow clicking on same button
    }
  }

  createEffect(() => {
    const value = currValue();
    const option = maybeToEntries(props.options).find(([k]) => k === value);
    if (!option) return;

    const [, info] = option;

    if (typeof info === "string") {
      setSelectTitle(props.tooltip);
    } else {
      setSelectTitle(`${props.tooltip}\n\n${info.name}: ${info.tooltip}`);
    }
  });

  createEffect(() => {
    if (props.customTitle) {
      select.value = ""; // Set to placeholder so it can be used as a context menu
    }
  });

  return (
    <div
      class={"flex items-center justify-between factorio-form-element factorio-select-container " + (props.customTitle ? "" : "mt-1 mb-1")}
      aria-disabled={props.formArgs?.disabled || props.disabled}
    >
      <Show when={props.name && !props.customTitle}>
        <label class="block text-white-500" for={props.key}>
          <span classList={{ "opacity-50": props.disabled }}>{props.name}</span>
          <FactorioTooltip name={props.name!} tooltip={props.tooltip} splash={props.splash} options={maybeToEntries(props.options)} />
        </label>
      </Show>

      <div class="relative overflow-hidden">
        <Show when={props.customTitle}>{props.customTitle}</Show>

        <select
          {...selectProps}
          ref={select}
          id={props.key}
          name={props.key}
          onChange={handleChange}
          title={selectTitle()}
          class={props.customTitle ? "no-icon absolute inset-0 h-8 w-8 p-0 m-0 cursor-pointer opacity-50" : undefined}
        >
          <Show when={props.customTitle}>
            {/* In context menu mode, we use an empty option as utility to reset the value */}
            <option value="" disabled hidden />
          </Show>

          <For each={maybeToEntries(props.options)}>
            {([k, v]) => (
              <option
                title={typeof v === "string" ? undefined : v.tooltip}
                value={k}
                disabled={(v as any)?.disabled}
                selected={k === currValue()}
              >
                {typeof v === "string" ? v : v.name}
              </option>
            )}
          </For>
        </select>
      </div>
    </div>
  );
}
