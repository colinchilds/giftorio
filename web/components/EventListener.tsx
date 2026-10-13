import { onCleanup, onMount } from "solid-js";

export interface EventListenerProps<K extends keyof DocumentEventMap> {
  event: K;
  handler(event: DocumentEventMap[K]): void;
}
export default function EventListener<K extends keyof DocumentEventMap>(props: EventListenerProps<K>) {
  const { event, handler } = props;
  onMount(() => {
    document.addEventListener(event, handler);
    onCleanup(() => document.removeEventListener(event, handler));
  });
  return null;
}
