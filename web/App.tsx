import getAnimationInfo, { AnimationInfo, getRawImageData, UnsupportedAnimationError } from "@/animationInfo";
import Background, { BackgroundApi } from "@/components/Background";
import FactorioDialogDelete, { FactorioDialogDeleteApi } from "@/components/FactorioDialogDelete";
import FactorioSelect, { SelectOption } from "@/components/FactorioSelect";
import FactorioSlider from "@/components/FactorioSlider";
import FactorioToasts, { FactorioToastsApi } from "@/components/FactorioToasts";
import FactorioTooltip from "@/components/FactorioTooltip";
import KeyboardListener from "@/components/KeyboardListener";
import SignalPresetSelect from "@/components/signal/SignalPresetSelect";
import SignalSelectors, { TimingSignals } from "@/components/signal/SignalSelectors";
import { FORM_ELEMENTS, getInitialValues, LAST_FILE_KEY, setInitialValues, SHOW_ADVANCED_KEY, SignalPreset } from "@/data";
import { loadFileDB, saveFileDB } from "@/db";
import { FormElementValue } from "@/types";
import { formatDuration, formatFileSize, kebabCaseToSentenceCase, useErrM } from "@/utils";
import { createEffect, createMemo, createSignal, For, onMount, Show } from "solid-js";
import { createStore } from "solid-js/store";
import { Portal } from "solid-js/web";
import { renderFrame } from "./worker";

export interface AppApi {
  showOpenStreamingWindowWarning(): void;
}
export interface AppProps {
  ref(api: AppApi): void;
  streamingAvailable: boolean;
  worker: Worker;
}
export default function App({ ref, streamingAvailable, worker }: AppProps) {
  // State
  const [formData, setFormData] = createStore(getInitialValues());
  const [currFile, setCurrFile] = createSignal<File | null>(null);
  const [animationInfo, setAnimationInfo] = createSignal<Partial<AnimationInfo>>();
  const [isGenerating, setIsGenerating] = createSignal(false);
  const [isDragging, setIsDragging] = createSignal(false);
  const [offset, setOffset] = createSignal({ x: 0, y: 0 });
  const [showAdvanced, setShowAdvanced] = createSignal(false);
  const [isMobile, setIsMobile] = createSignal(false);
  const [imageData, setImageData] = createSignal<ImageData>();
  const [workerShouldShowPreview, setWorkerShouldShowPreview] = createSignal(false);
  const [isStaticImage, setIsStaticImage] = createSignal(false);
  const [showPreview, setShowPreview] = createSignal(false);
  const [signalPreset, setSignalPreset] = createSignal<SignalPreset | null>(null);
  const [signalPresetRaw, setSignalPresetRaw] = createSignal<Uint8Array | null>(null);
  const [timingSignals, setTimingSignals] = createSignal<TimingSignals | null>(null);
  const [showStreamingWindowWarning, setShowStreamingWindowWarning] = createSignal(false);

  let canvas: OffscreenCanvas;
  let ctx: OffscreenCanvasRenderingContext2D;

  let refBackground: BackgroundApi = null!;
  let refCanvas: HTMLCanvasElement = null!; // Used for preview
  let refDialogReset: FactorioDialogDeleteApi = null!;
  let refFormContainer: HTMLDivElement = null!;
  let refToasts: FactorioToastsApi = null!;

  function showOpenStreamingWindowWarning() {
    setShowStreamingWindowWarning(true);
    setTimeout(() => setShowStreamingWindowWarning(false), 10000);
  }
  ref({ showOpenStreamingWindowWarning } satisfies AppApi);

  // Refs
  const formRefs: {
    blueprintResult: HTMLDivElement;
    blueprintStatus: HTMLDivElement;
    fileInput: HTMLInputElement;
    progressBar: HTMLDivElement;
    progressContainer: HTMLDivElement;
    progressStatus: HTMLDivElement;
    maxsize: HTMLInputElement;
    responseText: HTMLDivElement;
    submitButton: HTMLButtonElement;
  } = {} as any;

  function showToast(...args: Parameters<FactorioToastsApi["showToast"]>) {
    refToasts.showToast(...args);
  }

  function FormElement({ key, value }: { key: string; value: FormElementValue }) {
    const { disabled = false, name, tooltip, splash, type } = value;
    const tooltipProps = { name, tooltip, splash };

    if (type === "checkbox") {
      return (
        <div class="mb-1 flex factorio-form-element" aria-disabled={value.disabled}>
          <label class="checkbox-label">
            <input
              type="checkbox"
              class="sr-only"
              checked={!!formData[key as keyof typeof formData]}
              onChange={(e) => setFormData(key as keyof typeof formData, e.currentTarget.checked as any)}
            />
            <div class="checkbox"></div>
            <div class="ml-3 text-white-500">
              {name}
              <FactorioTooltip {...tooltipProps} />
            </div>
          </label>
        </div>
      );
    } else if (type === "select") {
      const k = key as keyof typeof FORM_ELEMENTS;
      const { options: _opt } = value as unknown as { options: [string, SelectOption][] | Record<string, SelectOption> };
      const options: [string, SelectOption][] = Array.isArray(_opt) ? _opt : Object.entries(_opt);
      return (
        <FactorioSelect
          ref={(e) => ((formRefs as any)[k] = e)}
          formArgs={value}
          initialValue={formData[key as keyof typeof formData] as string}
          key={k}
          onChange={(v) => setFormData(k, v)}
          options={options}
          {...tooltipProps}
        />
      );
    } else if (type === "number") {
      const v = value as unknown as { min?: number; max?: number; step?: number; values?: number[]; compact?: boolean };
      const compact = v.compact ?? false;

      function handleSubmit(rawValue: number) {
        const ref = formRefs[key as keyof typeof formRefs] as HTMLInputElement;
        if (v.values && !v.values.includes(rawValue)) {
          ref.setCustomValidity(`Value must be one of: ${v.values.join(", ")}`);
          ref.setAttribute("aria-invalid", "true");
          return;
        }
        ref.setCustomValidity("");
        ref.setAttribute("aria-invalid", "false");
        setFormData(key as keyof typeof formData, rawValue);
      }

      return (
        <div class="mb-1 factorio-form-element" aria-disabled={value.disabled} classList={{ flex: compact }}>
          <label class="block text-white-500 mb-0 w-full">
            {name}
            <FactorioTooltip {...tooltipProps} />
          </label>
          <div class="items-center gap-3 w-full">
            <FactorioSlider
              compact={compact}
              id={key}
              disabled={disabled}
              ref={(e) => ((formRefs as any)[key] = e)}
              value={+formData[key as keyof typeof formData]!}
              onSubmit={handleSubmit}
              {...(v as any)}
            />
          </div>
        </div>
      );
    }
  }

  // Worker message handler
  worker.onmessage = async ({ data }) => {
    if (data.progress) {
      const { percentage, status } = data.progress;
      formRefs.progressBar.style.setProperty("--progress", `${percentage}%`);
      formRefs.progressStatus.textContent = `(${percentage.toFixed(0)}%)  ${status}`;
    } else if (data.shouldShowPreview) {
      const { showPreview } = data.shouldShowPreview;
      setWorkerShouldShowPreview(showPreview);
    } else if (data.blueprintMetadata) {
      const { blueprintMetadata } = data;

      showToast("Blueprint downloaded!");

      formRefs.progressContainer.classList.add("hidden");
      formRefs.blueprintResult.classList.remove("hidden");
      formRefs.responseText.innerHTML = "Blueprint downloaded!";
      formRefs.submitButton.disabled = false;
    } else if (data.preview) {
      const { pixels, width, height } = data.preview;
      renderFrame(canvas, ctx, pixels, width, height);
      // const url = await rgbaToImageURL(pixels, width, height);
      // refBackground.setImageURL(url);
    } else if (data.error) {
      showToast(data.error, { isError: true });
      setIsGenerating(false);
      formRefs.submitButton.disabled = false;
    }
  };

  async function setInputFile(file: File | null) {
    if (!file) {
      setCurrFile(null);
      saveFileDB(null, LAST_FILE_KEY);
      refBackground.setImageURL(null);
      return;
    }

    let isStaticImage: boolean | null = null;
    let data: ImageData | undefined = undefined;

    try {
      const info = getAnimationInfo(new Uint8Array(await file.arrayBuffer()));
      setAnimationInfo(info);
      isStaticImage = false;
    } catch (e: any) {
      if (!(e instanceof UnsupportedAnimationError)) throw e;

      data = await getRawImageData(file);
      isStaticImage = true;

      // Parse the image directly in JS for more compatibility (we decode using the browser and
      // send the image buffer directly to Rust instead of relying on support from Rust crates).
      setAnimationInfo(new AnimationInfo(0, 0, data.width, data.height));
    }

    setIsStaticImage(isStaticImage);
    setImageData(data);
    setCurrFile(file);
    saveFileDB(file, LAST_FILE_KEY);

    refBackground.setImageURL(URL.createObjectURL(file));
  }

  async function handleSubmit(evt: SubmitEvent) {
    evt.preventDefault();

    // Validate ALL inputs (instead of only showing errors for the first invalid input)
    let isValid = true;
    for (const e of [evt.target, ...Object.values(formRefs)]) {
      if ((e instanceof HTMLFormElement || e instanceof HTMLInputElement) && e.checkValidity() === false) {
        e.reportValidity();
        isValid = false;
      }
    }
    if (!isValid) {
      showToast("Please fix all errors", { isError: true });
      return;
    }

    setIsGenerating(true);
    formRefs.submitButton.disabled = true;

    // Reset UI state
    formRefs.blueprintStatus.classList.remove("hidden");
    formRefs.progressContainer.classList.remove("hidden");
    formRefs.blueprintResult.classList.add("hidden");

    // Reset progress bar and status text explicitly
    formRefs.progressBar.style.setProperty("--progress", "0%");
    formRefs.progressStatus.textContent = "Starting...";

    if (formData.mode === "full" && !currFile()) {
      showToast("Please select a file", { isError: true });
      setIsGenerating(false);
      formRefs.submitButton.disabled = false;
      return;
    }

    try {
      const data = currFile()
        ? imageData()
          ? new Uint8Array(imageData()!.data.buffer, imageData()!.data.byteOffset, imageData()!.data.byteLength)
          : new Uint8Array(await currFile()!.arrayBuffer())
        : new Uint8Array();
      let signalCompression = null;

      if (formData.signalCompressionType === "delta") {
        signalCompression = "delta";
      } else if (formData.signalCompressionType === "temporal") {
        signalCompression = { temporal: { window: +formData.temporalCompressionWindow } };
      }

      function getMode() {
        if (!currFile() || !isStaticImage()) return formData.mode;
        return {
          staticImage: {
            useConstantCombinators: formData.staticImageMode === "constant",
            useCombinators: formData.staticImageMode !== "lamps",
            useTimer: formData.staticImageMode === "video",
          },
        };
      }

      worker.postMessage({
        generate: {
          imageData: data,
          args: {
            customHeight: +formData.customHeight,
            customWidth: +formData.customWidth,
            displayMarginY: +formData.displayMarginY,
            flippedAxes: `${formData.flippedAxes}`,
            grayscaleBits: +formData.grayscaleBits,
            imageMetadata: {
              imageType: isStaticImage() ? "" : currFile()?.type.substring(6 /* image/ */),
              imageSize: imageData() && [imageData()!.width, imageData()!.height],
            },
            imageFilters: [],
            imageRotation: `${formData.imageRotation}`,
            includePowerPoles: formData.includePowerPoles,
            maxGroupSize: +formData.maxGroupSize! || null,
            maxSize: +formData.maxSize,
            mode: getMode(),
            name: currFile()?.name,
            outputFormat: `${formData.outputFormat}`,
            powerPoleName: formData.powerPoleName,
            powerPoleQuality: formData.powerPoleQuality,
            resamplingFilter: `${formData.resamplingFilter}`,
            showPreview: showPreview(),
            signalCompression,
            signalSorting:
              formData.signalSorting === "auto" ? (formData.outputFormat === "json" ? "json" : "compression") : `${formData.signalSorting}`,
            targetFps: +formData.targetFps,
            timingSignals: timingSignals(),
            useGreenLampWires: !!(formData.wireColor === "green"),
            useHorizontalLampWires: !!(formData.connectionDirection === "horizontal"),
          },
        },
      });
    } catch (err: any) {
      console.error("Failed to process file:", err);
      showToast(`Failed to process file: ${err}`, { isError: true });
      setIsGenerating(false);
      formRefs.submitButton.disabled = false;
    }
  }

  function handleMouseDown(e: MouseEvent) {
    e.preventDefault();
    const r = refFormContainer.getBoundingClientRect();
    setOffset({ x: e.clientX - r.left, y: e.clientY - r.top });
    (e.target! as HTMLElement).style.cursor = "grabbing";
    setIsDragging(true);

    document.addEventListener("mouseup", handleMouseUp);
  }

  function handleMouseUp(e: MouseEvent) {
    if (isDragging()) {
      e.preventDefault();
      (e.target! as HTMLElement).style.cursor = "pointer";
      setIsDragging(false);
    }

    document.removeEventListener("mouseup", handleMouseUp);
  }

  document.addEventListener("mousemove", (e) => {
    if (isDragging()) {
      refFormContainer.style.position = "absolute";
      refFormContainer.style.left = `${e.clientX - offset().x}px`;
      refFormContainer.style.top = `${e.clientY - offset().y}px`;
    }
  });

  onMount(() => {
    // Check if device is mobile
    const IS_MOBILE_WIDTH_THRESHOLD = 768;
    setIsMobile(window.innerWidth <= IS_MOBILE_WIDTH_THRESHOLD);
    window.addEventListener("resize", () => setIsMobile(window.innerWidth <= IS_MOBILE_WIDTH_THRESHOLD));

    const bounds = refFormContainer.getBoundingClientRect();
    refFormContainer.style.position = "absolute";
    refFormContainer.style.left = `${bounds.left}px`;
    refFormContainer.style.top = `${bounds.top}px`;
  });

  onMount(() => {
    // Show advanced after first render to preserver original centering
    setShowAdvanced(localStorage.getItem(SHOW_ADVANCED_KEY) === "true");
    loadFileDB(LAST_FILE_KEY)
      .then((file) => {
        if (!file) return;
        const dataTransfer = new DataTransfer();
        dataTransfer.items.add(file);
        formRefs.fileInput.files = dataTransfer.files; // Pre-populate form with file
        setInputFile(file);
      })
      .catch((err) => {
        console.error("Failed to load file:", err);
        showToast(`Failed to load file: ${err}`, { isError: true });
      });
  });

  onMount(() => {
    canvas = refCanvas.transferControlToOffscreen();
    ctx = canvas.getContext("2d")!;
  });

  createEffect(() => {
    if (formData.mode !== "full") {
      formRefs.fileInput.setAttribute("aria-disabled", "true");
    } else {
      formRefs.fileInput.removeAttribute("aria-disabled");
    }
    setInitialValues(formData);
  });
  createEffect(() => localStorage.setItem(SHOW_ADVANCED_KEY, showAdvanced().toString()));
  createEffect(() => signalPresetRaw() && worker.postMessage({ signalPresetRaw: signalPresetRaw()! }));
  createEffect(() => worker.postMessage({ showPreview: { showPreview: showPreview() } }));

  function AnimationInfoDisplay(props: Partial<AnimationInfo> & { file: File | null }) {
    return (
      <div class="text-gray-300">
        <div class="flex items-center justify-between">
          <Show when={props.width && props.height}>
            <div>
              <span class="font-semibold">{props.width}</span> x <span class="font-semibold">{props.height}</span> px
            </div>
          </Show>
          <Show when={props.duration}>
            <div>
              Length: <span class="font-semibold">{formatDuration(props.duration!)}</span>
            </div>
          </Show>
        </div>
        <div class="flex items-center justify-between">
          <Show when={props.file}>
            <div>
              <span>{formatFileSize(props.file!.size)}</span>
            </div>
          </Show>
          <Show when={props.frames && props.duration}>
            <div>
              Frames: <span class="font-semibold">{props.frames}</span> (~
              {Math.round((10 * (props.frames! * 1000)) / props.duration!) / 10} FPS)
            </div>
          </Show>
        </div>
      </div>
    );
  }
  function FormElementsDisplay(props: { isStaticImage: boolean }) {
    const disallowedKeys = ["includePowerPoles", "grayscaleBits", "outputFormat"];
    const visibleFormElements = createMemo(() =>
      Object.entries(FORM_ELEMENTS).map(([key, value]) => {
        if (disallowedKeys.includes(key)) return null;
        else if (["mode", "customWidth", "customHeight"].includes(key)) return null;
        else if (key === "staticImageMode" && !props.isStaticImage) return null;
        else if (key === "temporalCompressionWindow" && formData.signalCompressionType !== "temporal") return null;
        else if (formData.mode === "full" && (key === "customWidth" || key === "customHeight")) return null;
        else if (formData.mode !== "full") {
          const allowed = [
            "connectionDirection",
            "flippedAxes",
            "grayscaleBits",
            "imageRotation",
            "maxGroupSize",
            "outputFormat",
            "signalSorting",
            "substationQuality",
            "useDLC",
            "wireColor",
          ];
          if (!allowed.includes(key)) {
            value = { ...value, disabled: true } as any;
          }
        } else if (props.isStaticImage) {
          const { staticImageMode: mode } = formData;
          const disallowedKeys = [
            //
            "includeLastFrame",
            "mode",
            "signalCompressionType",
            "temporalCompressionWindow",
            "targetFps",
          ]; // Remove booleans / falsey values
          const disabledKeys = [
            mode === "lamps" && "connectionDirection",
            mode === "lamps" && "grayscaleBits",
            mode === "lamps" && "maxGroupSize",
            mode === "lamps" && "signalSorting",
            mode === "lamps" && "wireColor",
          ].map((k) => (k === !!k || !k ? null : k)); // Remove booleans / falsey values

          if (disallowedKeys.includes(key)) {
            return null;
          } else if (disabledKeys.includes(key)) {
            value = { ...value, disabled: true } as any;
          }
        }
        return [key, value] as const;
      }),
    );
    return (
      <For each={visibleFormElements().filter((v) => !!v)}>
        {/* Use our custom form element parser to dynamically generate the elements */}
        {([key, value]) => <FormElement {...{ key, value }} />}
      </For>
    );
  }

  return (
    <>
      <KeyboardListener keys={["Enter", "e", "E"]} onKeyDown={() => formRefs.submitButton.click()} />
      <Background ref={(api) => (refBackground = api)} />

      <div classList={{ hidden: !showPreview() }}>
        <div class="fixed inset-0 bg-black w-full h-full" />
        <canvas ref={refCanvas!} class="fixed inset-0 w-full h-full object-contain" />

        <Show
          when={workerShouldShowPreview()}
          fallback={
            <>
              <div class="fixed inset-0 bg-black w-full h-full opacity-50" />
              <h2 class="fixed inset-0 grid place-items-center [text-shadow:1px_1px_2px_black]">Waiting for preview...</h2>
            </>
          }
        >
          <h2 class="fixed bottom-[20%] left-1/2 -translate-x-1/2 [text-shadow:1px_1px_2px_black]">Live Preview</h2>
        </Show>
      </div>

      <div class="fixed top-0 left-0 right-0 font-semibold">
        <Show when={isMobile()}>
          <div class="mobile-warning">⚠️ GIFtorio works best on desktop devices. Features may be limited on mobile.</div>
        </Show>
        <Show when={!streamingAvailable}>
          <div class="streaming-unavailable-warning">
            ⚠️ Streaming unavailable in this browser. This may cause issues with larger files.
          </div>
        </Show>

        <div
          class="streaming-unavailable-warning transition-opacity duration-300"
          classList={{ "opacity-0": !showStreamingWindowWarning(), "opacity-100": showStreamingWindowWarning() }}
        >
          ⚠️ Opened a new window to download the file as the download stream appeared to be stuck. If you're on Firefox, try soft refreshing
          the page (Ctrl + R).
        </div>
      </div>
      <div class="flex flex-col items-center justify-start min-h-screen">
        <FactorioToasts ref={(api) => (refToasts = api)} />
        <hr class="h-[20vh]" />
        <div ref={refFormContainer!} class="panel-container flex">
          <div classList={{ hidden: isGenerating() }} class="panel form flex-shrink-0">
            <div class="flex items-center justify-between">
              <h2>GIF/WebP to Blueprint</h2>
              <div class="handle cursor-pointer" onMouseDown={handleMouseDown}></div>
              <div
                class="mb-[10px] w-5 h-5 flex items-center content-center justify-center"
                classList={{
                  "panel-inset-orange": showAdvanced(),
                  "text-black": showAdvanced(),
                  "panel-inset-light": !showAdvanced(),
                  "text-white": !showAdvanced(),
                }}
                onClick={() => setShowAdvanced((x) => !x)}
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke-width="1.5"
                  stroke="currentColor"
                  class="size-4"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z"
                  />
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
                </svg>
              </div>
            </div>

            <form onSubmit={handleSubmit} id="main" class="panel-inset-light bg-gray-500 p-6 rounded shadow-md w-full min-w-100 max-w-md">
              <div class="mb-4 flex items-center justify-between">
                {/* File Input */}
                <div class="factorio-form-element">
                  <input
                    ref={(el) => (formRefs.fileInput = el)}
                    class="text-white-500 w-full focus:outline-none focus:ring"
                    type="file"
                    id="gifInput"
                    required={formData.mode === "full"}
                    onChange={(el) => {
                      const errM = useErrM(el.currentTarget);
                      setInputFile(el.target.files![0])
                        .catch((e) => {
                          errM.report(e.toString());
                          el.target.value = "";
                          showToast(e.toString(), { isError: true });
                        })
                        .finally(() => errM.test());
                    }}
                    accept="image/*"
                  />
                </div>
              </div>
              <Show when={animationInfo()} fallback={"Loading..."}>
                <AnimationInfoDisplay {...animationInfo()!} file={currFile()} />
              </Show>

              {/* Max Size Input */}
              <div class="mt-5 flex items-center justify-between factorio-form-element" aria-disabled={formData.mode !== "full"}>
                <label class="text-white-500" for="maxsize">
                  Max Size
                  <FactorioTooltip
                    tooltip="Maximum size of the longest side (length or width) of the output image in tiles. Larger values create higher resolution blueprints but take longer to generate and import, and can negatively impact on game performance."
                    name="Max Size"
                    splash="Blueprint size increases x4 for a x2 increase in max size."
                  />
                </label>

                <input
                  ref={(el) => (formRefs.maxsize = el)}
                  class="bg-gray-100 focus:bg-tan-500 w-24 px-3 py-2 border rounded focus:outline-none focus:ring"
                  type="number"
                  id="maxsize"
                  onInput={(e) => setFormData("maxSize", +e.target.value)}
                  value={formData.maxSize}
                  min="2"
                  max="7680"
                />
              </div>

              {(formData.mode === "lamps" || formData.mode === "lampGrid") && (
                <>
                  <FormElement key="customHeight" value={FORM_ELEMENTS.customHeight} />
                  <FormElement key="customWidth" value={FORM_ELEMENTS.customWidth} />
                </>
              )}

              <SignalPresetSelect setSignalPreset={setSignalPreset} setSignalPresetRaw={setSignalPresetRaw} showToast={showToast} />
              <For each={["grayscaleBits", "outputFormat"]}>
                {(k) => <FormElement key={k} value={FORM_ELEMENTS[k as keyof typeof FORM_ELEMENTS]} />}
              </For>

              <div class="mb-4" />

              {/* Advanced Settings and Submit Buttons */}
              <div>
                <div class="flex items-center justify-between">
                  <button class="button bg-gray-100 px-4" type="button" onClick={() => setShowAdvanced(!showAdvanced())}>
                    Settings
                  </button>
                  <button class="button button-green-right" ref={(el) => (formRefs.submitButton = el)} id="submit" type="submit">
                    Generate
                  </button>
                </div>
              </div>
            </form>
          </div>

          <div class="panel w-90 z-10" classList={{ hidden: !showAdvanced() || isGenerating() }}>
            <div class="flex items-center justify-between">
              <h3>Advanced Options</h3>
              <div class="handle cursor-pointer" onMouseDown={handleMouseDown} onMouseUp={handleMouseUp}></div>
              {/* <button class="button button-red icon-button w-8 h-8 self-start" type="button">
                ↻
              </button> */}
            </div>
            <div class="panel-inset-light px-3 pt-2 py-1 shadow-md w-full max-w-md overflow-y-auto" style={{ "max-height": "50vh" }}>
              <FormElement key="includePowerPoles" value={FORM_ELEMENTS.includePowerPoles} />

              <Show when={signalPreset()}>
                <FactorioSelect
                  name="Pole Quality"
                  key="includePowerPoles"
                  title="The quality level of the power poles."
                  disabled={!formData.includePowerPoles}
                  options={
                    signalPreset()!.qualities.length
                      ? [
                          ["<default>", { name: "(Default)", tooltip: `Alias for "${signalPreset()!.qualities[0]}"` }],
                          ...signalPreset()!.qualities.map((q) => {
                            const poleData = signalPreset()!.electric_poles;
                            const targetName = formData.powerPoleName;
                            const { supply_area_distance: dist } = (
                              poleData.find((d) => d.name === targetName) ?? poleData[0]!
                            ).supply_area_distances.find((v) => v.quality === q)!;

                            const tooltip = `${dist * 2}x${dist * 2} Supply area`;
                            return [q, { name: kebabCaseToSentenceCase(q), tooltip }] as const;
                          }),
                        ]
                      : []
                  }
                  tooltip="The quality level of the power poles."
                  splash={
                    signalPreset()!.qualities.length === 0
                      ? "Irrelevant for the current signal preset."
                      : `Above values are for the ${kebabCaseToSentenceCase(formData.powerPoleName)}.`
                  }
                  initialValue={formData.powerPoleQuality ?? "<default>"}
                  onChange={(v) => setFormData("powerPoleQuality", v as any)}
                />
                <FactorioSelect
                  name="Pole Type"
                  key="powerPoleName"
                  disabled={!formData.includePowerPoles}
                  options={signalPreset()!.electric_poles.map(({ name, supply_area_distances: dists, ...p }) => {
                    const targetQual = formData.powerPoleQuality;
                    const dist = (dists.find((d) => !targetQual || d.quality === targetQual) ?? dists[0])?.supply_area_distance;
                    const tooltip = `${dist * 2}x${dist * 2} Supply area, ${p.tile_width}x${p.tile_height} Footprint`;
                    return [name, { name: kebabCaseToSentenceCase(name), tooltip }] as const;
                  })}
                  tooltip="The name of the electric pole entity to use to power the blueprint."
                  splash={
                    signalPreset()!.electric_poles.length === 0
                      ? "Irrelevant for current signal preset."
                      : `Above values are for the "${kebabCaseToSentenceCase(formData.powerPoleQuality!)}" quality.`
                  }
                  initialValue={formData.powerPoleName}
                  onChange={(v) => setFormData("powerPoleName", v as any)}
                />
              </Show>

              <FormElementsDisplay isStaticImage={isStaticImage()} />

              <hr class="my-2" />
              <h2>Advanced</h2>

              <Show when={signalPreset()}>
                <SignalSelectors
                  values={{
                    f: { type: "virtual", name: "signal-F", quality: "normal" },
                    s: { type: "virtual", name: "signal-B", quality: "normal" },
                    t: { type: "virtual", name: "signal-T", quality: "normal" },
                  }}
                  signalData={signalPreset()!}
                  onChange={(v) => setTimingSignals(v)}
                />
              </Show>

              <hr class="my-2" />
              <h2>Data</h2>

              <button class="button button-red icon-button w-full h-8 self-start" type="button" onClick={() => refDialogReset.show()}>
                Reset all data
              </button>

              <Portal>
                <FactorioDialogDelete
                  ref={(api) => (refDialogReset = api)}
                  onConfirm={() => {
                    indexedDB
                      .databases()
                      .then((dbs) => Promise.all(dbs.map(({ name }) => name && indexedDB.deleteDatabase(name))))
                      .then(() => {
                        localStorage.clear();
                        window.location.reload();
                      });
                  }}
                >
                  <h2>Are you sure you want to delete all data?</h2>
                  <div class="text-white-500 text-center">
                    This will remove all signal presets, configuration data, and reset all options back to their default values.
                  </div>
                </FactorioDialogDelete>
              </Portal>
            </div>
          </div>

          {/* Blueprint Status Section */}
          <div ref={(el) => (formRefs.blueprintStatus = el)} classList={{ hidden: !isGenerating() }} class="panel w-full min-w-[384px] ">
            <div class="progress-container" ref={(el) => (formRefs.progressContainer = el)} id="progressContainer">
              <p ref={(el) => (formRefs.progressStatus = el)} id="progressStatus" class="progress-status text-tan-500">
                Starting...
              </p>
              <div class="progress-bar-container">
                <div class="progress-bar-wrapper">
                  <div ref={(el) => (formRefs.progressBar = el)} id="progressBar" class="progress-bar"></div>
                </div>
              </div>
              <div class="my-5" />
              <button type="button" class="button w-full" onClick={() => setShowPreview((v) => !v)}>
                Click to {showPreview() ? "hide" : "show"} preview
              </button>
            </div>
            <div
              ref={(el) => (formRefs.blueprintResult = el)}
              id="blueprintResult"
              class="p-3 mb-3"
              classList={{ hidden: !isGenerating() }}
            >
              <h2>Blueprint Download</h2>
              <div
                ref={(el) => (formRefs.responseText = el)}
                id="responseText"
                class="panel-inset-light text-gray-100 p-3 border rounded mb-6 overflow-y-auto overflow-x-hidden whitespace-pre-wrap break-all h-32"
              ></div>
              <div class="flex items-center justify-between">
                <button onClick={() => setIsGenerating(false)} id="backButton" class="button">
                  Back
                </button>
              </div>
              <div class="mt-6 text-center text-white-500">
                <p>
                  Liking GIFtorio? Consider{" "}
                  <a
                    href="https://www.buymeacoffee.com/colinchilds"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="text-bright-green-500 hover:text-tan-500"
                  >
                    supporting the original creator
                  </a>
                  !
                </p>
              </div>
            </div>
          </div>
        </div>

        <footer class="fixed bottom-8 w-full gap-8 text-center text-gray-300">
          <a href="https://github.com/Number-3434/giftorio" class="pr-8" target="_blank" rel="noopener noreferrer">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 30 30"
              width="1em"
              height="1em"
              fill="currentColor"
              class="inline-block align-middle mr-1.5"
            >
              <path d="M15 3C8.373 3 3 8.373 3 15c0 5.623 3.872 10.328 9.092 11.63a1.8 1.8 0 0 1-.092-.583v-2.051h-1.508c-.821 0-1.551-.353-1.905-1.009-.393-.729-.461-1.844-1.435-2.526-.289-.227-.069-.486.264-.451.615.174 1.125.596 1.605 1.222.478.627.703.769 1.596.769.433 0 1.081-.025 1.691-.121.328-.833.895-1.6 1.588-1.962-3.996-.411-5.903-2.399-5.903-5.098 0-1.162.495-2.286 1.336-3.233-.276-.94-.623-2.857.106-3.587 1.798 0 2.885 1.166 3.146 1.481A9 9 0 0 1 15.495 9c1.036 0 2.024.174 2.922.483C18.675 9.17 19.763 8 21.565 8c.732.731.381 2.656.102 3.594.836.945 1.328 2.066 1.328 3.226 0 2.697-1.904 4.684-5.894 5.097C18.199 20.49 19 22.1 19 23.313v2.734c0 .104-.023.179-.035.268C23.641 24.676 27 20.236 27 15c0-6.627-5.373-12-12-12"></path>
            </svg>
            <span class="align-middle">Contribute or report an issue</span>
          </a>
          <a href="https://www.buymeacoffee.com/colinchilds" target="_blank" rel="noopener noreferrer">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              width="1em"
              height="1em"
              fill="currentColor"
              class="inline-block align-middle mr-1.5"
            >
              <path d="M23.881 8.948c-.773-4.085-4.859-4.593-4.859-4.593H.723c-.604 0-.679.798-.679.798s-.082 7.324-.022 11.822c.164 2.424 2.586 2.672 2.586 2.672s8.267-.023 11.966-.049c2.438-.426 2.683-2.566 2.658-3.734 4.352.24 7.422-2.831 6.649-6.916m-11.062 3.511c-1.246 1.453-4.011 3.976-4.011 3.976s-.121.119-.31.023c-.076-.057-.108-.09-.108-.09-.443-.441-3.368-3.049-4.034-3.954-.709-.965-1.041-2.7-.091-3.71.951-1.01 3.005-1.086 4.363.407 0 0 1.565-1.782 3.468-.963s1.832 3.011.723 4.311m6.173.478c-.928.116-1.682.028-1.682.028V7.284h1.77s1.971.551 1.971 2.638c0 1.913-.985 2.667-2.059 3.015"></path>
            </svg>
            <span class="align-middle">Support</span>
          </a>
        </footer>
      </div>
    </>
  );
}
