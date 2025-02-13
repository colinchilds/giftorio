import { onMount, For, createSignal } from "solid-js";
import nyanMp4 from "./assets/img/nyan.mp4";
import nyanWebm from "./assets/img/nyan.webm";
import rickMp4 from "./assets/img/rick.mp4";
import rickWebm from "./assets/img/rick.webm";
import gumpMp4 from "./assets/img/gump.mp4";
import gumpWebm from "./assets/img/gump.webm";


const VIDS = [
  {
    mp4: nyanMp4,
    webm: nyanWebm,
  },
  {
    mp4: rickMp4,
    webm: rickWebm,
  },
  {
    mp4: gumpMp4,
    webm: gumpWebm,
  },
];

function Background(props) {
  const [currentVideo, setCurrentVideo] = createSignal(0);

  onMount(() => {
    setCurrentVideo(Math.floor(Math.random() * VIDS.length))
    let duration = props.interval || 10000;

    setInterval(() => setCurrentVideo((currentVideo() + 1) % VIDS.length), duration);
  });

  return (
    <div id="video-container" class="fixed top-0 left-0 w-full h-full -z-1">
      <For each={VIDS}>
        {(vid, i) => (
          <video
            id={`bg-video-${i() + 1}`}
            class="absolute top-0 left-0 w-full h-full object-cover transition-opacity duration-1000 ease-in-out pointer-events-none opacity-0"
            classList={{ "opacity-100": i() === currentVideo() }}
            autoplay
            muted
            loop
            playsinline
          >
            <source src={vid.mp4} type="video/mp4" />
            <source src={vid.webm} type="video/webm" />
          </video>
        )}
      </For>
    </div>
  );
}

export default Background;
