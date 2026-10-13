import { createSignal, For, onCleanup, onMount } from "solid-js";

export interface ToastArgs {
  isError?: boolean;
}
export interface ToastsApi {
  showToast(msg: string, args?: ToastArgs): void;
}
export interface ToastsProps {
  ref(api: ToastsApi): void;
}
export default function Toasts(props: ToastsProps) {
  const [toasts, setToasts] = createSignal<Record<number, ToastArgs & { msg: string } & { x: number; y: number }>>({});

  let nextToastId = 0;
  let mousePos = { x: 0, y: 0 };

  function showToast(msg: string, args?: ToastArgs) {
    const id = nextToastId++;
    setToasts((prev) => ({ ...prev, [id]: { msg, ...args, ...mousePos } }));
    setTimeout(() => {
      setToasts((prev) => {
        const { [id]: _, ...rest } = prev;
        return rest;
      });
    }, 3000);
  }

  function onMouseMove(e: MouseEvent) {
    const { x, y } = e;
    mousePos = { x, y };
  }

  onMount(() => {
    window.addEventListener("mousemove", onMouseMove);

    onCleanup(() => {
      window.removeEventListener("mousemove", onMouseMove);
    });
  });

  props.ref({ showToast });

  return (
    <For each={Object.values(toasts())}>
      {(t) => {
        let el!: HTMLDivElement;

        onMount(() => {
          el.showPopover(); // Show above e.g. other dialogs

          const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);

          const MARGIN = 1.5 * rem;
          const SPEED = 2.5 * rem;

          // Horizontal
          const halfW = el.offsetWidth / 2;
          const x = Math.min(Math.max(t.x, halfW + MARGIN), window.innerWidth - halfW - MARGIN);

          // Vertical
          const desiredY = t.y - MARGIN - el.offsetHeight;
          const startY = Math.max(MARGIN, desiredY);
          const distance = startY - MARGIN;

          if (distance > 0) {
            el.style.setProperty("--duration", `${distance / SPEED}s`);
          } else {
            el.style.animation = "none";
          }

          el.style.left = `${x}px`;
          el.style.overflowWrap = "break-word";
          el.style.top = `${startY}px`;
        });

        return (
          <div ref={el} popover="manual" class="fixed toast select-none">
            {t.msg}
          </div>
        );
      }}
    </For>
  );
}
