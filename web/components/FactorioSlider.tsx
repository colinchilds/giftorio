import { createMemo, createSignal } from "solid-js";

export interface FactorioSliderProps {
  compact?: boolean;
  value: number;
  disabled?: boolean;
  values?: number[];
  id?: string;
  min?: number;
  max?: number;
  scale?: "linear" | "ln10";
  step?: number;
  onChange?(value: number): void;
  onSubmit?(value: number): void;
  ref?(el: HTMLInputElement): void;
}
export default function FactorioSlider(props: FactorioSliderProps) {
  let { values, min = 0, max = Infinity, step } = props;
  const hasCustomValues = createMemo(() => !!props.values);
  const [currValue, setCurrValue] = createSignal(props.value);

  function valueToPosition(value: number) {
    const { values } = props;

    if (!values) return value;
    if (value <= values[0]) return 0;
    if (value >= values[values.length - 1]) return values.length - 1;

    for (let i = 1; i < values.length; i++) {
      if (value <= values[i]) {
        const a = values[i - 1];
        const b = values[i];
        const t = (value - a) / (b - a);
        return i - 1 + t;
      }
    }
  }
  function positionToValue(position: number) {
    const { values } = props;

    if (!values) return position;
    if (position <= 0) return values[0];
    if (position >= values.length - 1) return values[values.length - 1];

    const i = Math.floor(position);
    const t = position - i;

    const a = values[i];
    const b = values[i + 1];

    return a + (b - a) * t;
  }

  if (props.scale === "ln10") {
    values = [];
    for (let mag = min; mag <= max; mag *= 10) {
      for (let i = 0; i < 10; i++) {
        const v = (i + 1) * mag;
        if (v >= min && v <= max) values.push(v);
      }
    }
    values = [...new Set(values)].sort((a, b) => a - b);
  }
  if (values) {
    step = undefined;
  }

  return (
    <div class="factorio-slider-container flex justify-between items-center gap-3 flex-1" classList={{ "w-full": true }}>
      <div
        aria-disabled={props.disabled}
        class="factorio-slider-wrapper flex-1"
        role="slider"
        style={{ "--slider-width": hasCustomValues() ? "15px" : "25px" }}
        classList={{ "has-multiple-values": hasCustomValues() }}
      >
        {(() => {
          const v = valueToPosition(currValue())!;
          const progress = values ? v / (values.length - 1) : (v - min) / (max - min);
          return (
            <>
              <div
                class="factorio-slider-thumb"
                aria-disabled={props.disabled}
                classList={{ "has-multiple-values": hasCustomValues() }}
                style={{ "--progress": `${progress}` }}
              />
              <div class="factorio-slider-fill" aria-disabled={props.disabled} style={{ width: `${progress * 100}%` }} />
            </>
          );
        })()}
        <div>
          {hasCustomValues() && values && (
            <div class="indicators">
              {values.map(() => (
                <div class="indicator" />
              ))}
            </div>
          )}
          <input
            disabled={props.disabled}
            type="range"
            id={props.id}
            class="factorio-slider"
            value={valueToPosition(currValue())!}
            min={values ? 0 : min}
            max={values ? values.length - 1 : max}
            step={values ? 1 : step}
            onInput={(e) => {
              setCurrValue(positionToValue(+e.target.value));
              props.onChange?.(currValue());
            }}
            onChange={(e) => {
              e.preventDefault();
              props.onChange?.(currValue());
              props.onSubmit?.(currValue());
            }}
          />
        </div>
      </div>
      <input
        ref={props.ref}
        disabled={props.disabled}
        type="number"
        class="bg-gray-100 focus:bg-tan-500 w-20 px-3 py-2 border rounded focus:outline-none focus:ring"
        value={currValue()}
        min={values ? values[0] : min}
        max={values ? values[values.length - 1] : max}
        step={step}
        onInput={(e) => {
          const value = +e.target!.value;
          const input = e.currentTarget;
          let errMsg: string | null = null;

          if (values && hasCustomValues()) {
            if (!values.includes(value)) errMsg = `Value must be one of: ${values.join(", ")}`;
          } else if (!hasCustomValues) {
            const currMin = values ? values[0] : min;
            const currMax = values ? values[values.length - 1] : max;
            if (value < currMin || value > currMax) errMsg = `Value (${value}) must be between ${currMin} and ${currMax}`;
          }
          if (errMsg) {
            input.setCustomValidity(errMsg);
            input.setAttribute("aria-invalid", "true");
            input.reportValidity();
            return;
          }
          input.setCustomValidity("");
          input.setAttribute("aria-invalid", "false");

          setCurrValue(value);
          props.onChange?.(value);
          props.onSubmit?.(+e.target!.value);
        }}
        placeholder={`${values ? values[0] : min}-${values ? values[values.length - 1] : max}`}
      />
    </div>
  );
}
