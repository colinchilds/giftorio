import { createSignal } from 'solid-js';
import bgUrl from './assets/img/background.png';


function App() {
  const [isGenerating, setIsGenerating] = createSignal(false);
  const [isDlc, setIsDlc] = createSignal(false);
  const worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });

  let toggleInput, toggleBg, toggleLabel, dot, substationQualityDiv, responseText, blueprintStatus, blueprintTitle, progressContainer, progressBar, progressStatus, submitButton, blueprintResult;

  worker.onmessage = (event) => {
    if (event.data.progress) {
      const { percentage, status } = event.data.progress;

      // Update the progress bar and status text
      progressBar.style.width = percentage + "%";
      progressStatus.textContent = status;
    } else if (event.data.blueprint) {
      const { blueprint } = event.data;
      //setIsGenerating(false);

      // Update UI to show the completed blueprint
      blueprintTitle.textContent = "Generated Blueprint";
      progressContainer.classList.add("hidden"); // hide progress UI
      blueprintResult.classList.remove("hidden");  // show blueprint result
      responseText.innerHTML = blueprint;

      submitButton.disabled = false;
    }
  };

  const toggleChange = () => {
    const isChecked = toggleInput.checked;
    toggleLabel.textContent = isChecked ? 'Yes' : 'No';
    toggleBg.classList.toggle('bg-dark-gray-500', isChecked);
    toggleBg.classList.toggle('bg-light-gray-500', !isChecked);
    setIsDlc(isChecked);

    if (isChecked) {
      dot.style.transform = 'translateX(100%)';
    } else {
      dot.style.transform = 'translateX(0)';
    }
  }

  const toClipboard = () => {
    navigator.clipboard.writeText(responseText.innerText)
      .then(() => {
        console.log('Blueprint copied to clipboard!');
      })
      .catch((err) => {
        console.error('Failed to copy blueprint:', err);
      });
  };

  const handleSubmit = async (event) => {
    event.preventDefault();
    setIsGenerating(true);
    submitButton.disabled = true;

    // Show the blueprint status section and reset progress UI
    blueprintStatus.classList.remove("hidden");
    blueprintTitle.textContent = "Generating Blueprint...";
    progressContainer.classList.remove("hidden");
    blueprintResult.classList.add("hidden");
    progressBar.style.width = "0%";
    progressStatus.textContent = "Starting...";

    const gifInput = document.getElementById("gifInput");
    const file = gifInput.files[0];
    const targetFps = document.getElementById("framerate").value;
    const maxSize = document.getElementById("maxsize").value;
    const useDlc = isDlc();
    const substationQuality = document.getElementById("substationQuality").value;

    if (file && file.type === "image/gif") {
      const buff = await file.arrayBuffer();
      const gifData = new Uint8Array(buff);
      worker.postMessage({ gifData, targetFps, maxSize, useDlc, substationQuality });
    }
  };

  return (
    <div style={{"background-image": `url(${bgUrl})`}} class="bg-cover bg-gray-500 flex items-center justify-center min-h-screen">
        <form
          onSubmit={handleSubmit}
          id="blueprint_form"
          classList={{hidden: isGenerating()}}
          class="bg-gray-500 gray-box-border p-6 rounded shadow-md w-full max-w-md">
            <div class="mb-4">
                <label class="block text-tan-500 text-sm font-bold mb-2" for="fileInput">Select File</label>
                <input
                    class="w-full px-3 py-2 border rounded focus:outline-none focus:ring"
                    type="file"
                    id="gifInput"
                    name="gifInput"
                    required
                />
            </div>
            <div class="mb-4">
                <label class="block text-tan-500 text-sm font-bold mb-2" for="framerate">Framerate</label>
                  <input
                        class="bg-very-light-gray-500 focus:bg-tan-500 w-full px-3 py-2 border rounded focus:outline-none focus:ring"
                        type="number"
                        id="framerate"
                        name="framerate"
                        placeholder="Enter max framerate (won't exceed original)"
                        value="30"
                  />
            </div>
            <div class="mb-4">
                <label class="block text-tan-500 text-sm font-bold mb-2" for="maxwidth">Max Width</label>
                <input
                    class="bg-very-light-gray-500 focus:bg-tan-500 w-full px-3 py-2 border rounded focus:outline-none focus:ring"
                    type="number"
                    id="maxsize"
                    name="maxsize"
                    placeholder="Enter max width"
                    value="100"
                />
            </div>
            <div class="mb-4 flex items-center">
                <span class="mr-4 text-tan-500 font-bold">Space Age?</span>
                <label for="toggle" class="flex items-center cursor-pointer">
                    <div class="relative">
                        <input ref={toggleInput} onChange={toggleChange} id="toggle" type="checkbox" name="toggle" class="sr-only" />
                        <div ref={toggleBg} id="togglebg" class="bg-very-light-gray-500 block w-14 h-8 rounded-full"></div>
                        <div ref={dot} class="dot absolute left-1 top-1 bg-white w-6 h-6 rounded-full transition-transform"></div>
                    </div>
                    <div ref={toggleLabel} class="ml-3 text-tan-500 font-medium" id="toggleLabel">No</div>
                </label>
            </div>
            <div id="quality" class="mb-4" classList={{hidden: !isDlc()}}>
                <label class="block text-tan-500 text-sm font-bold mb-2" for="substationQuality">Substation Quality</label>
                <select ref={substationQualityDiv} id="substationQuality" name="substationQuality" class="bg-very-light-gray-500 w-full px-3 py-2 border rounded focus:outline-none focus:ring">
                    <option value="normal">Normal</option>
                    <option value="uncommon">Uncommon</option>
                    <option value="rare">Rare</option>
                    <option value="epic">Epic</option>
                    <option value="legendary">Legendary</option>
                </select>
            </div>
            <div class="flex items-center justify-between">
                <button
                    class="bg-green-500 hover:bg-bright-green-500 text-black font-bold py-2 px-4"
                    ref={submitButton}
                    id="submit"
                    type="submit">
                    Submit
                </button>
            </div>
        </form>

        <div ref={blueprintStatus} id="blueprintStatus" class="bg-white p-6 rounded shadow-md w-full max-w-md" classList={{hidden: !isGenerating()}}>
            <h2 ref={blueprintTitle} id="blueprintTitle" class="text-xl font-bold mb-2"></h2>
            <div ref={progressContainer} id="progressContainer">
                <div class="w-full bg-gray-200 rounded-full h-4 mb-2">
                    <div ref={progressBar} id="progressBar" class="bg-blue-500 h-4 rounded-full" style="width: 0%"></div>
                </div>
                <p ref={progressStatus} id="progressStatus" class="text-gray-700">Starting...</p>
            </div>
            <div ref={blueprintResult} id="blueprintResult" classList={{hidden: !isGenerating()}}>
                <div ref={responseText} id="responseText" class="bg-gray-50 p-4 border rounded overflow-auto max-h-64 mb-2"></div>
                <button onClick={toClipboard} id="copyButton" class="bg-green-500 hover:bg-bright-green-500 text-white font-bold py-1 px-2 rounded">
                    Copy
                </button>
            </div>
        </div>
    </div>
  );
}

            // <div
            //     id="quality"
            //     class="mb-4 transition-all ease-in-out transform[translateY(0)] opacity-0"
            //     classList={{
            //       'opacity-0': !isDlc(),
            //       'transform[translateY(-100%)]': !isDlc(),
            //       'opacity-100': isDlc(),
            //       'transform[translateY(0)]': isDlc(),
            //     }}
            // >


export default App;
