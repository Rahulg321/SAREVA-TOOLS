// Client-side PDF text extraction. The file never leaves the browser — only the
// extracted text is sent to the Worker. Text-based PDFs only (no OCR).
(function () {
  const PDFJS = "https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.min.js";
  const WORKER = "https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js";

  const input = document.querySelector('input[type="file"][data-pdf]');
  const textarea = document.querySelector('textarea[name="text"]');
  const status = document.getElementById("file-status");
  if (!input || !textarea || !status) return;

  const loadScript = (src) =>
    new Promise((resolve, reject) => {
      if (window.pdfjsLib) return resolve();
      const s = document.createElement("script");
      s.src = src;
      s.onload = resolve;
      s.onerror = () => reject(new Error("Could not load the PDF engine."));
      document.head.appendChild(s);
    });

  input.addEventListener("change", async () => {
    const file = input.files && input.files[0];
    if (!file) return;
    status.textContent = "Reading PDF…";
    try {
      await loadScript(PDFJS);
      window.pdfjsLib.GlobalWorkerOptions.workerSrc = WORKER;
      const pdf = await window.pdfjsLib.getDocument({ data: await file.arrayBuffer() }).promise;
      let text = "";
      for (let page = 1; page <= pdf.numPages; page++) {
        status.textContent = `Extracting page ${page} / ${pdf.numPages}…`;
        const content = await (await pdf.getPage(page)).getTextContent();
        text += content.items.map((item) => item.str).join(" ") + "\n\n";
      }
      text = text.replace(/[ \t]+/g, " ").trim();
      if (!text) {
        status.textContent = "No text found — this looks like a scanned PDF (OCR not supported).";
        return;
      }
      textarea.value = text;
      status.textContent = `Extracted ${text.length.toLocaleString()} characters from ${pdf.numPages} pages.`;
    } catch (err) {
      status.textContent = err.message || "Could not read this PDF.";
    }
  });
})();
