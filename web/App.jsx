import { createSignal, createEffect } from 'solid-js';
import bgUrl from './assets/img/background.png';

// Constants
const INITIAL_VALUES = {
  framerate: 15,
  maxSize: 50,
  substationQualities: ['none', 'normal', 'uncommon', 'rare', 'epic', 'legendary']
};

function App() {
  // State
  const [isGenerating, setIsGenerating] = createSignal(false);
  const [isDlc, setIsDlc] = createSignal(false);
  const [progress, setProgress] = createSignal({ percentage: 0, status: 'Starting...' });
  const [blueprintData, setBlueprintData] = createSignal({ title: '', content: '' });
  const [toast, setToast] = createSignal({ show: false, message: '', isError: false });
  
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
  const toggleChange = () => {
    const isChecked = formRefs.toggleInput.checked;
    const currentQuality = formRefs.substationQuality.value;
    
    formRefs.toggleLabel.textContent = isChecked ? 'Yes' : 'No';
    formRefs.toggleBg.classList.toggle('bg-dark-gray-500', isChecked);
    formRefs.toggleBg.classList.toggle('bg-light-gray-500', !isChecked);
    
    // Store the current quality before state update
    if (!isChecked && ['uncommon', 'rare', 'epic', 'legendary'].includes(currentQuality)) {
      // Set timeout to run after the reactive updates
      setTimeout(() => {
        formRefs.substationQuality.value = 'normal';
      }, 0);
    }
    
    setIsDlc(isChecked);
    formRefs.dot.style.transform = isChecked ? 'translateX(100%)' : 'translateX(0)';
  };

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

    const formData = {
      file: formRefs.gifInput.files[0],
      targetFps: formRefs.framerate.value,
      maxSize: formRefs.maxsize.value,
      useDlc: isDlc(),
      substationQuality: formRefs.substationQuality?.value || 'normal'
    };

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
        useDlc: formData.useDlc,
        substationQuality: formData.substationQuality
      });
    } catch (err) {
      console.error('Failed to process file:', err);
      setToast({ show: true, message: 'Failed to process file', isError: true });
      setTimeout(() => setToast({ show: false, message: '', isError: false }), 3000);
      setIsGenerating(false);
      formRefs.submitButton.disabled = false;
    }
  };

  return (
    <div style={{"background-image": `url(${bgUrl})`}} class="bg-cover bg-gray-500 flex items-center justify-center min-h-screen">
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

      <div classList={{hidden: isGenerating()}} class="panel">
        <h2 class="text-tan-500">Convert GIF to Blueprint</h2>
        <form onSubmit={handleSubmit} class="panel-inset bg-gray-500 p-6 rounded shadow-md w-full max-w-md">
          {/* File Input */}
          <div class="mb-4">
            <label class="block text-tan-500 text-sm font-bold mb-2" for="gifInput">Select File</label>
            <input
              ref={el => formRefs.gifInput = el}
              class="bg-very-light-gray-500 w-full px-3 py-2 border rounded focus:outline-none focus:ring"
              type="file"
              id="gifInput"
              required
            />
          </div>

          {/* Framerate Input */}
          <div class="mb-4">
            <label class="block text-tan-500 text-sm font-bold mb-2" for="framerate">Framerate</label>
            <input
              ref={el => formRefs.framerate = el}
              class="bg-very-light-gray-500 focus:bg-tan-500 w-full px-3 py-2 border rounded focus:outline-none focus:ring"
              type="number"
              id="framerate"
              value={INITIAL_VALUES.framerate}
              placeholder="Enter max framerate (won't exceed original)"
            />
          </div>

          {/* Max Size Input */}
          <div class="mb-4">
            <label class="block text-tan-500 text-sm font-bold mb-2" for="maxsize">Max Width</label>
            <input
              ref={el => formRefs.maxsize = el}
              class="bg-very-light-gray-500 focus:bg-tan-500 w-full px-3 py-2 border rounded focus:outline-none focus:ring"
              type="number"
              id="maxsize"
              value={INITIAL_VALUES.maxSize}
              min="2"
              max="300"
            />
          </div>

          {/* Toggle Input */}
          <div class="mb-4 flex items-center">
            <span class="mr-4 text-tan-500 font-bold">Space Age?</span>
            <label for="toggle" class="flex items-center cursor-pointer">
              <div class="relative">
                <input ref={el => formRefs.toggleInput = el} onChange={toggleChange} id="toggle" type="checkbox" name="toggle" class="sr-only" />
                <div ref={el => formRefs.toggleBg = el} id="togglebg" class="bg-very-light-gray-500 block w-14 h-8 rounded-full"></div>
                <div ref={el => formRefs.dot = el} class="dot absolute left-1 top-1 bg-white w-6 h-6 rounded-full transition-transform"></div>
              </div>
              <div ref={el => formRefs.toggleLabel = el} class="ml-3 text-tan-500 font-medium" id="toggleLabel">No</div>
            </label>
          </div>

          {/* Substation Quality Select */}
          <div class="mb-4">
            <label class="block text-tan-500 text-sm font-bold mb-2" for="substationQuality">Substation Quality</label>
            <select 
              ref={el => formRefs.substationQuality = el}
              id="substationQuality" 
              name="substationQuality" 
              class="bg-very-light-gray-500 w-full px-3 py-2 border rounded focus:outline-none focus:ring"
              defaultValue="normal"
            >
              <option value="normal">Normal</option>
              {isDlc() && (
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

          {/* Submit Button */}
          <div class="flex items-center justify-end">
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

      {/* Blueprint Status Section */}
      <div ref={el => formRefs.blueprintStatus = el} 
           classList={{hidden: !isGenerating()}} 
           class="panel p-6 rounded shadow-md w-full max-w-md">
        <h2 ref={el => formRefs.blueprintTitle = el} id="blueprintTitle" class="text-tan-500 text-xl font-bold mb-2"></h2>
        <div ref={el => formRefs.progressContainer = el} id="progressContainer">
          <div class="w-full bg-dark-gray-500 rounded-full h-4 mb-2">
            <div ref={el => formRefs.progressBar = el} id="progressBar" class="bg-green-500 h-4 rounded-full" style="width: 0%"></div>
          </div>
          <p ref={el => formRefs.progressStatus = el} id="progressStatus" class="text-tan-500">Starting...</p>
        </div>
        <div ref={el => formRefs.blueprintResult = el} id="blueprintResult" classList={{hidden: !isGenerating()}}>
          <div ref={el => formRefs.responseText = el} id="responseText" class="panel-inset-light text-very-light-gray-500 p-3 border rounded overflow-auto max-h-64 mb-6"></div>
          <div class="flex items-center justify-between">
            <button onClick={() => setIsGenerating(false)} id="backButton" class="button">
              Back
            </button>
            <button onClick={toClipboard} id="copyButton" class="button button-green">
              Copy
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
