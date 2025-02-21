import { onMount, For, createSignal } from "solid-js";
import nyanMp4 from "./assets/img/nyan.mp4";
import nyanWebm from "./assets/img/nyan.webm";
import nyanGif from "./assets/img/nyan.gif";
import rickMp4 from "./assets/img/rick.mp4";
import rickWebm from "./assets/img/rick.webm";
import gumpMp4 from "./assets/img/gump.mp4";
import gumpWebm from "./assets/img/gump.webm";

const MEDIA = [
  {
    mp4: nyanMp4,
    webm: nyanWebm,
    fallback: nyanGif
  },
  {
    mp4: rickMp4,
    webm: rickWebm,
    fallback: nyanGif
  },
  {
    mp4: gumpMp4,
    webm: gumpWebm,
    fallback: nyanGif
  }
];

function Background(props) {
  const [currentMedia, setCurrentMedia] = createSignal(0);
  const [useVideoBackground, setUseVideoBackground] = createSignal(true);

  onMount(() => {
    // Check if video playback is supported
    const video = document.createElement('video');
    setUseVideoBackground(!!video.canPlayType);

    setCurrentMedia(Math.floor(Math.random() * MEDIA.length));
    const duration = props.interval || 10000;
    setInterval(() => setCurrentMedia((currentMedia() + 1) % MEDIA.length), duration);
  });

  return (
    <div id="media-container" class="fixed top-0 left-0 w-full h-full" style="z-index: -1;">
      <div class="absolute inset-0 bg-black"></div>
      {useVideoBackground() ? (
        <For each={MEDIA}>
          {(media, i) => (
            <video
              id={`bg-video-${i() + 1}`}
              class="absolute top-0 left-0 w-full h-full object-cover transition-opacity duration-1000 ease-in-out pointer-events-none opacity-0"
              classList={{ "opacity-100": i() === currentMedia() }}
              autoplay
              muted
              loop
              playsinline
              style="z-index: 0;"
            >
              <source src={media.mp4} type="video/mp4" />
              <source src={media.webm} type="video/webm" />
              <img src={media.fallback} alt="" class="w-full h-full object-cover" />
            </video>
          )}
        </For>
      ) : (
        <For each={MEDIA}>
          {(media, i) => (
            <img
              src={media.fallback}
              alt=""
              class="absolute top-0 left-0 w-full h-full object-cover transition-opacity duration-1000 ease-in-out opacity-0"
              classList={{ "opacity-100": i() === currentMedia() }}
              style="z-index: 0;"
            />
          )}
        </For>
      )}
    </div>
  );
}

export default Background;
