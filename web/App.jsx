import { createSignal, onMount, createEffect } from 'solid-js';
import { createStore } from 'solid-js/store';
import Background from './Background';
import infoIcon from "./assets/img/info.png";

// Constants
const INITIAL_VALUES = {
  file: null,
  targetFps: 15,
  maxSize: 50,
  useDLC: false,
  substationQualities: ['none', 'normal', 'uncommon', 'rare', 'epic', 'legendary'],
  substationQuality: 'normal',
  grayscaleBits: 0,
};

function App() {
  // State
  const [formData, setFormData] = createStore({...INITIAL_VALUES});
  const [isGenerating, setIsGenerating] = createSignal(false);
  const [progress, setProgress] = createSignal({ percentage: 0, status: 'Starting...' });
  const [blueprintData, setBlueprintData] = createSignal({ title: '', content: '' });
  const [toast, setToast] = createSignal({ show: false, message: '', isError: false });
  const [isDragging, setIsDragging] = createSignal(false);
  const [xOffset, setXOffset] = createSignal(0);
  const [yOffset, setYOffset] = createSignal(0);
  const [showAdvanced, setShowAdvanced] = createSignal(false);
  let form;

  // Worker setup
  const worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });

  // Refs
  let formRefs = {};

  // Worker message handler
  worker.onmessage = (event) => {
    if (event.data.progress) {
      const { percentage, status } = event.data.progress;
      setProgress({ percentage, status });
      formRefs.progressBar.style.width = `${percentage}%`;
      formRefs.progressStatus.textContent = status;
    } else if (event.data.blueprint) {
      const { blueprint } = event.data;
      setBlueprintData({ title: 'Generated Blueprint', content: blueprint });
      formRefs.progressContainer.classList.add("hidden");
      formRefs.blueprintResult.classList.remove("hidden");
      formRefs.responseText.innerHTML = blueprint;
      formRefs.submitButton.disabled = false;
    } else if (event.data.error) {
      setToast({ show: true, message: event.data.error, isError: true });
      setTimeout(() => setToast({ show: false, message: '', isError: false }), 3000);
      setIsGenerating(false);
      formRefs.submitButton.disabled = false;
    }
  };

  // Event handlers
  const toClipboard = async () => {
    try {
      await navigator.clipboard.writeText(formRefs.responseText.innerText);
      setToast({ show: true, message: 'Copied to clipboard!', isError: false });
      setTimeout(() => setToast({ show: false, message: '', isError: false }), 2000);
    } catch (err) {
      console.error('Failed to copy blueprint:', err);
      setToast({ show: true, message: 'Failed to copy to clipboard', isError: true });
      setTimeout(() => setToast({ show: false, message: '', isError: false }), 2000);
    }
  };

  const handleSubmit = async (event) => {
    event.preventDefault();
    setIsGenerating(true);
    formRefs.submitButton.disabled = true;

    // Reset UI state
    formRefs.blueprintStatus.classList.remove("hidden");
    setBlueprintData({ title: 'Generating Blueprint...', content: '' });
    formRefs.progressContainer.classList.remove("hidden");
    formRefs.blueprintResult.classList.add("hidden");
    setProgress({ percentage: 0, status: 'Starting...' });
    
    // Reset progress bar and status text explicitly
    formRefs.progressBar.style.width = '0%';
    formRefs.progressStatus.textContent = 'Starting...';

    if (!formData.file) {
      setToast({ show: true, message: 'Please select a file', isError: true });
      setTimeout(() => setToast({ show: false, message: '', isError: false }), 3000);
      setIsGenerating(false);
      formRefs.submitButton.disabled = false;
      return;
    }

    if (formData.file.type !== "image/gif") {
      setToast({ show: true, message: 'Please select a GIF file', isError: true });
      setTimeout(() => setToast({ show: false, message: '', isError: false }), 3000);
      setIsGenerating(false);
      formRefs.submitButton.disabled = false;
      return;
    }

    try {
      const gifData = new Uint8Array(await formData.file.arrayBuffer());
      worker.postMessage({
        gifData,
        targetFps: formData.targetFps,
        maxSize: formData.maxSize,
        useDLC: formData.useDLC,
        substationQuality: formData.substationQuality,
        grayscaleBits: formData.grayscaleBits,
      });
    } catch (err) {
      console.error('Failed to process file:', err);
      setToast({ show: true, message: 'Failed to process file', isError: true });
      setTimeout(() => setToast({ show: false, message: '', isError: false }), 3000);
      setIsGenerating(false);
      formRefs.submitButton.disabled = false;
    }
  };

  const handleMouseDown = (e) => {
    e.preventDefault();
    setXOffset(e.clientX - form.getBoundingClientRect().left);
    setYOffset(e.clientY - form.getBoundingClientRect().top);
    e.target.style.cursor = 'grabbing';
    setIsDragging(true);
  };

  const handleMouseUp = (e) => {
    e.preventDefault();
    e.target.style.cursor = 'pointer';
    setIsDragging(false);
  };

  document.addEventListener('mousemove', (e) => {
    if (isDragging()) {
      form.style.position = 'absolute';
      form.style.left = `${e.clientX - xOffset()}px`;
      form.style.top = `${e.clientY - yOffset()}px`;
    }
  });

  onMount(() => {
      const bounds = form.getBoundingClientRect();
      form.style.position = 'absolute';
      form.style.left = `${bounds.left}px`;
      form.style.top = `${bounds.top}px`;
  });

  createEffect(() => {
    const triggers = document.querySelectorAll('.tooltip-trigger');
    
    triggers.forEach(trigger => {
      trigger.addEventListener('mousemove', (e) => {
        const tooltip = trigger.nextElementSibling;
        const rect = trigger.getBoundingClientRect();
        
        const vpWidth = window.innerWidth;
        const vpHeight = window.innerHeight;
        const tooltipRect = tooltip.getBoundingClientRect();
        
        let x = e.clientX + 10;
        let y = e.clientY + 10;
        
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
      });
    });
  });

  return (
    <>
    <Background />
    <div class="flex flex-col items-center justify-center min-h-screen">
      <div 
        classList={{
          "opacity-0": !toast().show,
          "opacity-100": toast().show,
          "bg-green-500": !toast().isError,
          "bg-red-500": toast().isError
        }}
        class="fixed top-4 right-4 text-white px-4 py-2 rounded shadow-lg transition-opacity duration-300"
      >
        {toast().message}
      </div>

      <div ref={form} class="panel-container flex">
        <div classList={{hidden: isGenerating()}} class="panel form">
          <div class="flex items-center justify-between">
            <h2 class="text-tan-500">Convert GIF to Blueprint</h2>
            <div class="handle cursor-pointer" onMouseDown={handleMouseDown} onMouseUp={handleMouseUp}></div>
            <div
              class="mb-[10px] w-5 h-5 flex items-center content-center justify-center"
              classList={{
                'panel-inset-orange': showAdvanced(),
                'text-black': showAdvanced(),
                'panel-inset-light': !showAdvanced(),
                'text-white': !showAdvanced(),
              }}
              onClick={() => setShowAdvanced(!showAdvanced())}
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z" />
                <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
              </svg>
            </div>
          </div>
          <form onSubmit={handleSubmit} class="panel-inset-light bg-gray-500 p-6 rounded shadow-md w-full max-w-md">
            {/* File Input */}
            <div class="mb-4">
              <input
                ref={el => formRefs.gifInput = el}
                class="text-white-500 w-full focus:outline-none focus:ring"
                type="file"
                id="gifInput"
                required
                onChange={e => setFormData('file', e.target.files[0])}
                accept="image/gif"
              />
            </div>

            {/* Max Size Input */}
            <div class="mb-4">
              <label class="block text-white-500 mb-2" for="maxsize">
                Max Size
                <img src={infoIcon} class="inline-block ml-1 mb-0.5 w-4 h-4 tooltip-trigger" alt="Info"/>
                <span class="tooltip">
                  Maximum size of the longest side (length or width) of the output image in tiles. Larger values create bigger blueprints but take longer to generate and may have a negative impact on game performance.
                </span>
              </label>
              <input
                ref={el => formRefs.maxsize = el}
                class="bg-gray-100 focus:bg-tan-500 w-full px-3 py-2 border rounded focus:outline-none focus:ring"
                type="number"
                id="maxsize"
                onInput={e => setFormData('maxSize', e.target.value)}
                value={formData.maxSize}
                min="2"
                max="300"
              />
            </div>


            {/* Advanced Settings and Submit Buttons */}
            <div class="flex items-center justify-between">
              <button
                class="button bg-gray-100 px-4"
                type="button"
                onClick={() => setShowAdvanced(!showAdvanced())}>
                Advanced Options
              </button>
              <button
                class="button button-green-right"
                ref={el => formRefs.submitButton = el}
                id="submit"
                type="submit">
                Generate
              </button>
            </div>
          </form>
        </div>

        <div class="panel" classList={{hidden: !showAdvanced() || isGenerating()}}>
          <div class="flex items-center justify-between">
            <h3 class="text-tan-500">Advanced Options</h3>
            <div class="handle cursor-pointer" onMouseDown={handleMouseDown} onMouseUp={handleMouseUp}></div>
          </div>
          <div class="panel-inset-light p-6 rounded shadow-md w-full max-w-md">
            {/* DLC toggle */}
            <div class="mb-4 flex">
              <label class="checkbox-label">
                <input
                  type="checkbox"
                  class="sr-only"
                  checked={formData.useDLC}
                  onChange={e => setFormData("useDLC", e.currentTarget.checked)} />
                <div class="checkbox"></div>
                <div class="ml-4 text-white-500">
                  Use Space Age DLC?
                  <img src={infoIcon} className="inline-block ml-1 mb-0.5 w-4 h-4 tooltip-trigger" alt="Info"/>
                  <span className="tooltip">
                  If enabled, this can increase the number of available signals and substations, reducing the number of combinators
                    needed in the blueprint. It also allows for higher quality substations.
                </span>
                </div>
              </label>
            </div>

            {/* Substation Quality Select */}
            <div class="mb-4">
              <label class="block text-white-500 mb-2" for="substationQuality">Substation Quality</label>
              <select
                  ref={el => formRefs.substationQuality = el}
                  id="substationQuality"
                  name="substationQuality"
                  class="bg-gray-100 w-full px-3 py-2 border rounded focus:outline-none focus:ring"
                value={formData.substationQuality}
                onChange={e => setFormData("substationQuality", e.currentTarget.value)}
              >
                <option value="normal">Normal</option>
                {formData.useDLC && (
                  <>
                    <option value="uncommon">Uncommon</option>
                    <option value="rare">Rare</option>
                    <option value="epic">Epic</option>
                    <option value="legendary">Legendary</option>
                  </>
                )}
                <option value="none">None</option>
              </select>
            </div>

            {/* Framerate Input */}
            <div class="mb-4">
              <label className="block text-white-500 mb-2" htmlFor="framerate">
                Framerate
                <img src={infoIcon} className="inline-block ml-1 mb-0.5 w-4 h-4 tooltip-trigger" alt="Info"/>
                <span className="tooltip">
                  Maximum framerate of the output blueprint. The blueprint will not exceed the original framerate of the GIF.
                  The higher the framerate, the more frames will be generated, increasing the size of the blueprint and impacting game performance.
                </span>
              </label>
              <input
                  ref={el => formRefs.framerate = el}
                  class="bg-gray-100 focus:bg-tan-500 w-full px-3 py-2 border rounded focus:outline-none focus:ring"
                  type="number"
                  id="framerate"
                  value={formData.targetFps}
                  onChange={e => setFormData('targetFps', e.target.value)}
                  placeholder="Enter max framerate (won't exceed original)"
              />
            </div>

            {/* Color Mode */}
            <div class="mb-4">
              <label class="block text-white-500 mb-2" for="grayscaleBits">
                Color Mode
                <img src={infoIcon} className="inline-block ml-1 mb-0.5 w-4 h-4 tooltip-trigger" alt="Info"/>
                <span className="tooltip">
                  Full color will try to match the original GIF colors. If the blueprint is too large, you can try using grayscale.
                  8-bit grayscale has 256 shades of gray and can reduce the blueprint size by 60-70%, while 4-bit grayscale has 16 shades of gray
                  and can reduce the blueprint size by up to 85%.
                </span>
              </label>
              <select
                  ref={el => formRefs.grayscaleBits = el}
                  id="grayscaleBits"
                  name="grayscaleBits"
                  class="bg-gray-100 w-full px-3 py-2 border rounded focus:outline-none focus:ring"
                  value={formData.grayscaleBits}
                  onChange={e => setFormData("grayscaleBits", parseInt(e.currentTarget.value))}
              >
                <option value="0">Full Color</option>
                <option value="8">8-bit Grayscale (256 shades)</option>
                <option value="4">4-bit Grayscale (16 shades)</option>
              </select>
            </div>
          </div>
        </div>

        {/* Blueprint Status Section */}
        <div ref={el => formRefs.blueprintStatus = el} 
             classList={{hidden: !isGenerating()}} 
             class="panel w-full max-w-md min-w-[384px]">
          <div ref={el => formRefs.progressContainer = el} id="progressContainer">
            <div class="w-full bg-dark-gray-500 rounded-full h-4 mb-2">
              <div ref={el => formRefs.progressBar = el} id="progressBar" class="bg-green-500 h-4 rounded-full" style="width: 0%"></div>
            </div>
            <p ref={el => formRefs.progressStatus = el} id="progressStatus" class="text-tan-500">Starting...</p>
          </div>
          <div ref={el => formRefs.blueprintResult = el} id="blueprintResult" classList={{hidden: !isGenerating()}}>
            <h2 class="text-tan-500">Blueprint</h2>
            <div 
              ref={el => formRefs.responseText = el} 
              id="responseText" 
              class="panel-inset-light text-gray-100 p-3 border rounded mb-6 overflow-y-auto overflow-x-hidden whitespace-pre-wrap break-all h-32"
            ></div>
            <div class="flex items-center justify-between">
              <button onClick={() => setIsGenerating(false)} id="backButton" class="button">
                Back
              </button>
              <button onClick={toClipboard} id="copyButton" class="button button-green">
                Copy
              </button>
            </div>
            <div class="mt-6 text-center text-white-500">
              <p>Liking GIFtorio? Consider <a 
                href="https://www.buymeacoffee.com/colinchilds" 
                target="_blank" 
                rel="noopener noreferrer"
                class="text-bright-green-500 hover:text-tan-500"
              >a small donation</a> to help with development costs!</p>
            </div>
          </div>
        </div>
      </div>

      <footer class="fixed bottom-8 w-full gap-8 text-center text-gray-300">
        <a href="https://github.com/colinchilds/giftorio" class="pr-8"  target="_blank" rel="noopener noreferrer">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 30 30" width="1em" height="1em" fill="currentColor" class="inline-block align-middle mr-1.5"><path d="M15 3C8.373 3 3 8.373 3 15c0 5.623 3.872 10.328 9.092 11.63a1.8 1.8 0 0 1-.092-.583v-2.051h-1.508c-.821 0-1.551-.353-1.905-1.009-.393-.729-.461-1.844-1.435-2.526-.289-.227-.069-.486.264-.451.615.174 1.125.596 1.605 1.222.478.627.703.769 1.596.769.433 0 1.081-.025 1.691-.121.328-.833.895-1.6 1.588-1.962-3.996-.411-5.903-2.399-5.903-5.098 0-1.162.495-2.286 1.336-3.233-.276-.94-.623-2.857.106-3.587 1.798 0 2.885 1.166 3.146 1.481A9 9 0 0 1 15.495 9c1.036 0 2.024.174 2.922.483C18.675 9.17 19.763 8 21.565 8c.732.731.381 2.656.102 3.594.836.945 1.328 2.066 1.328 3.226 0 2.697-1.904 4.684-5.894 5.097C18.199 20.49 19 22.1 19 23.313v2.734c0 .104-.023.179-.035.268C23.641 24.676 27 20.236 27 15c0-6.627-5.373-12-12-12"></path></svg>
          <span class="align-middle">Contribute or report an issue</span>
        </a>
        <a href="https://www.buymeacoffee.com/colinchilds" target="_blank" rel="noopener noreferrer">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" class="inline-block align-middle mr-1.5"><path d="M23.881 8.948c-.773-4.085-4.859-4.593-4.859-4.593H.723c-.604 0-.679.798-.679.798s-.082 7.324-.022 11.822c.164 2.424 2.586 2.672 2.586 2.672s8.267-.023 11.966-.049c2.438-.426 2.683-2.566 2.658-3.734 4.352.24 7.422-2.831 6.649-6.916m-11.062 3.511c-1.246 1.453-4.011 3.976-4.011 3.976s-.121.119-.31.023c-.076-.057-.108-.09-.108-.09-.443-.441-3.368-3.049-4.034-3.954-.709-.965-1.041-2.7-.091-3.71.951-1.01 3.005-1.086 4.363.407 0 0 1.565-1.782 3.468-.963s1.832 3.011.723 4.311m6.173.478c-.928.116-1.682.028-1.682.028V7.284h1.77s1.971.551 1.971 2.638c0 1.913-.985 2.667-2.059 3.015"></path></svg>
          <span class="align-middle">Support</span>
          </a>
        </footer>
    </div>
    </>
  );
}

export default App;
