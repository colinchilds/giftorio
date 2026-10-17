import gumpMp4 from "@/assets/img/gump.mp4";
import gumpWebm from "@/assets/img/gump.webm";
import nyanGif from "@/assets/img/nyan.gif";
import nyanMp4 from "@/assets/img/nyan.mp4";
import nyanWebm from "@/assets/img/nyan.webm";
import rickMp4 from "@/assets/img/rick.mp4";
import rickWebm from "@/assets/img/rick.webm";
import { createSignal, For, onMount, Show } from "solid-js";

const MEDIA = [
  { mp4: nyanMp4, webm: nyanWebm, fallback: nyanGif },
  { mp4: rickMp4, webm: rickWebm, fallback: nyanGif },
  { mp4: gumpMp4, webm: gumpWebm, fallback: nyanGif },
] as const;

export interface BackgroundApi {
  setImageURL(url: string | null | undefined): void;
}
export interface BackgroundProps {
  ref?(api: BackgroundApi): void;
  interval?: number;
}
export default function Background(props: BackgroundProps) {
  const [currMediaIdx, setCurrMediaIdx] = createSignal(0);
  const [canUseVideoBg, setCanUseVideoBg] = createSignal(true);
  const [useVideoBg, setUseVideoBg] = createSignal(true);
  const [customImageURL, setCustomImageURL] = createSignal<string>();

  let refImage: HTMLImageElement | null = null;

  props.ref?.({
    setImageURL(url) {
      if (!url) {
        setUseVideoBg(canUseVideoBg());
        setCustomImageURL(undefined);
        return;
      }
      setCustomImageURL(url);
      setUseVideoBg(false);
    },
  } satisfies BackgroundApi);

  onMount(() => {
    // Check if video playback is supported
    const video = document.createElement("video");
    const canPlay = !!video.canPlayType;
    setCanUseVideoBg(canPlay);
    setUseVideoBg(canPlay);

    setCurrMediaIdx(Math.floor(Math.random() * MEDIA.length));
    const duration = props.interval || 10000;
    setInterval(() => setCurrMediaIdx((currMediaIdx() + 1) % MEDIA.length), duration);
  });

  return (
    <div id="media-container" class="fixed top-0 left-0 w-full h-full z-[-1]">
      <div class="absolute inset-0 bg-black"></div>

      <Show
        when={!customImageURL()}
        fallback={
          <img
            ref={refImage!}
            src={customImageURL() ?? undefined}
            alt="test"
            width="20"
            class="absolute top-0 left-0 w-full h-full object-contain transition-opacity duration-1000 ease-in-out opacity-30 z-0"
            style={{ "image-rendering": "pixelated" }}
          />
        }
      >
        <For each={MEDIA}>
          {(media, i) => (
            <Show
              when={useVideoBg()}
              fallback={
                <img
                  src={media.fallback}
                  alt=""
                  class="absolute top-0 left-0 w-full h-full object-cover transition-opacity duration-1000 ease-in-out opacity-0 z-0"
                  classList={{ "opacity-100": i() === currMediaIdx() }}
                />
              }
            >
              <video
                id={`bg-video-${i() + 1}`}
                class="absolute top-0 left-0 w-full h-full object-cover transition-opacity duration-1000 ease-in-out pointer-events-none opacity-0 z-0"
                classList={{ "opacity-100": i() === currMediaIdx() }}
                autoplay
                muted
                loop
                playsinline
              >
                <source src={media.mp4} type="video/mp4" />
                <source src={media.webm} type="video/webm" />
                <img src={media.fallback} alt="" class="w-full h-full object-cover" />
              </video>
            </Show>
          )}
        </For>
      </Show>
    </div>
  );
}
