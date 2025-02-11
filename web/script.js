const worker = new Worker("worker.js", { type: "module" });
const form = document.getElementById("blueprint_form");

// Get UI elements for blueprint status
const blueprintStatus = document.getElementById("blueprintStatus");
const blueprintTitle = document.getElementById("blueprintTitle");
const progressContainer = document.getElementById("progressContainer");
const progressBar = document.getElementById("progressBar");
const progressStatus = document.getElementById("progressStatus");
const blueprintResult = document.getElementById("blueprintResult");
const responseText = document.getElementById("responseText");

worker.onmessage = function (event) {
  if (event.data.progress) {
    const { percentage, status } = event.data.progress;

    // Update the progress bar and status text
    progressBar.style.width = percentage + "%";
    progressStatus.textContent = status;
  } else if (event.data.blueprint) {
    const { blueprint } = event.data;
    const submitButton = document.getElementById("submit");
    const spinner = document.getElementById("spinner");

    // Update UI to show the completed blueprint
    blueprintTitle.textContent = "Generated Blueprint";
    progressContainer.classList.add("hidden"); // hide progress UI
    blueprintResult.classList.remove("hidden");  // show blueprint result
    responseText.innerHTML = blueprint;

    // Re-enable the submit button and hide spinner
    submitButton.disabled = false;
    spinner.classList.add("hidden");
    spinner.classList.remove("inline");
  }
};

form.addEventListener("submit", async (event) => {
  event.preventDefault();

  const submitButton = document.getElementById("submit");
  submitButton.disabled = true;
  const spinner = document.getElementById("spinner");
  spinner.classList.remove("hidden");
  spinner.classList.add("inline");

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
  const useDlc = document.getElementById("toggle").checked;
  const substationQuality = document.getElementById("substationQuality").value;

  if (file && file.type === "image/gif") {
    const buff = await file.arrayBuffer();
    const gifData = new Uint8Array(buff);
    worker.postMessage({ gifData, targetFps, maxSize, useDlc, substationQuality });
  }
});
