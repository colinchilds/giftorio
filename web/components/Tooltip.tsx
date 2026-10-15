import infoIcon from "@/assets/img/info.png";
import { SelectOption } from "@/components/FactorioSelect";
import { For, JSX, Show } from "solid-js";

export interface TooltipProps {
  name: string;
  tooltip?: JSX.Element;
  options?: (readonly [string, string | SelectOption])[];
  splash?: JSX.Element;
}
export default function Tooltip(props: TooltipProps) {
  return (
    <>
      <img
        role="note"
        src={infoIcon}
        class="inline-block ml-1 mb-0.5 w-4 h-4 tooltip-trigger"
        alt="Info"
        onMouseMove={(e) => {
          const trigger = e.currentTarget;
          const tooltip = trigger.nextElementSibling! as HTMLElement;
          const rect = trigger.getBoundingClientRect();

          const vpWidth = window.innerWidth;
          const vpHeight = window.innerHeight;
          const tooltipRect = tooltip.getBoundingClientRect();

          let x = e.clientX + 20;
          let y = e.clientY + 20;

          // Check if tooltip would go off-screen to the right
          if (x + tooltipRect.width > vpWidth) {
            x = e.clientX - tooltipRect.width - 10;
          }

          // Check if tooltip would go off-screen at the bottom
          if (y + tooltipRect.height > vpHeight) {
            y = e.clientY - tooltipRect.height - 10;
          }

          tooltip.style.left = `${x}px`;
          tooltip.style.top = `${y}px`;
        }}
      />
      <div class="tooltip" role="tooltip">
        <div class="tooltip-header">{props.name}</div>
        {props.tooltip && <div class="text-white font-light">{props.tooltip}</div>}
        <Show when={props.options}>
          <div class="mt-2">
            <ul class="list-none pl-2">
              <For each={props.options!}>
                {([_, v]) => {
                  if (typeof v === "string") return;
                  return (
                    <li class="before:mr-3 before:content-['•'] text-tan-500">
                      <span class="font-semibold  text-cyan-500">{v.name}:</span>{" "}
                      <span class="text-tan-500 font-semibold">{v.tooltip}</span>
                    </li>
                  );
                }}
              </For>
            </ul>
          </div>
        </Show>
        {props.splash && <div class="text-tan-500 mt-5 opacity-60">{props.splash}</div>}
      </div>
    </>
  );
}
