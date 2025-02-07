const worker = new Worker("worker.js", { type: "module" });
const form = document.getElementById("blueprint_form");

worker.onmessage = function(event) {
  const { blueprint } = event.data;
  const resultTag = document.getElementById("response");
  const resultText = document.getElementById("responseText");
  const submitButton = document.getElementById("submit");
  const spinner = document.getElementById("spinner");

  form.classList.add("hidden");
  submitButton.disabled = false;
  spinner.classList.add("hidden");
  spinner.classList.remove("inline");
  resultText.innerHTML = blueprint;
  resultTag.classList.remove("hidden");
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const submitButton = document.getElementById("submit");
  submitButton.disabled = true;

  const spinner = document.getElementById("spinner");
  spinner.classList.remove("hidden");
  spinner.classList.add("inline");

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
