import { zipSync } from "fflate";

// ── Constants ─────────────────────────────────────────────────────────────────
const MAX_FILES = 50;
const MAX_BYTES = 50 * 1024 * 1024; // 50 MB
const ACCEPTED_TYPES = new Set(["image/jpeg", "image/png"]);
const ACCEPTED_EXTS  = new Set(["jpg", "jpeg", "png"]);

// ── DOM refs ──────────────────────────────────────────────────────────────────
const dropZone       = document.getElementById("drop-zone");
const fileInput      = document.getElementById("file-input");
const controls       = document.getElementById("controls");
const progressEl     = document.getElementById("progress-counter");
const downloadAllBtn = document.getElementById("download-all-btn");
const resultsEl      = document.getElementById("results");
const srStatus       = document.getElementById("sr-status");
const batchWarning   = document.getElementById("batch-warning");
const howBtn         = document.getElementById("how-it-works-btn");
const modal          = document.getElementById("how-it-works-modal");
const modalClose     = document.getElementById("modal-close-btn");

// ── State ─────────────────────────────────────────────────────────────────────
const queue = new Map(); // id → { filename, state, cleanBuf? }
let idCounter = 0;
let doneCount = 0;
let completedBuffers = []; // { filename, buffer } for ZIP

// ── Worker ────────────────────────────────────────────────────────────────────
const worker = new Worker(new URL("./worker.js", import.meta.url), {
  type: "module",
});

worker.onmessage = ({ data }) => {
  if (data.type === "result") {
    const entry = queue.get(data.id);
    if (!entry) return;
    entry.state = "done";
    entry.cleanBuf = data.buffer;
    doneCount++;
    completedBuffers.push({ filename: `clean_${data.filename}`, buffer: data.buffer });
    updateProgress();
    renderResult(data.id, data.filename, {
      tags: data.tags,
      buffer: data.buffer,
      originalSize: data.originalSize,
      durationMs: data.durationMs,
    });
    downloadAllBtn.disabled = false;
  } else if (data.type === "error") {
    const entry = queue.get(data.id);
    if (entry) entry.state = "error";
    renderError(data.id, data.filename, data.message);
  }
};

// ── Drag & drop ───────────────────────────────────────────────────────────────
dropZone.addEventListener("dragover", (e) => {
  e.preventDefault();
  dropZone.classList.add("drag-over");
});
dropZone.addEventListener("dragleave", () => dropZone.classList.remove("drag-over"));
dropZone.addEventListener("drop", (e) => {
  e.preventDefault();
  dropZone.classList.remove("drag-over");
  handleFiles([...e.dataTransfer.files]);
});
dropZone.addEventListener("keydown", (e) => {
  if (e.key === "Enter" || e.key === " ") fileInput.click();
});
fileInput.addEventListener("change", () => handleFiles([...fileInput.files]));

// ── File handling ─────────────────────────────────────────────────────────────
function handleFiles(files) {
  if (files.length === 0) return;

  const remaining = MAX_FILES - queue.size;
  if (remaining <= 0) {
    showGlobalError(`Batch limit reached — only ${MAX_FILES} files per session.`);
    return;
  }

  let batch = files;
  if (files.length > remaining) {
    batchWarning.textContent = `Only the first ${remaining} file(s) from this selection will be processed (limit ${MAX_FILES} per session).`;
    batchWarning.hidden = false;
    batch = files.slice(0, remaining);
  } else {
    batchWarning.hidden = true;
    batchWarning.textContent = "";
  }

  let accepted = 0;
  for (const file of batch) {
    if (queue.size >= MAX_FILES) {
      showGlobalError(`Batch limit reached — only ${MAX_FILES} files per session.`);
      break;
    }
    const ext = file.name.split(".").pop().toLowerCase();
    if (!ACCEPTED_TYPES.has(file.type) && !ACCEPTED_EXTS.has(ext)) {
      renderRejected(file.name, "Not a JPEG or PNG file.");
      continue;
    }
    if (file.size > MAX_BYTES) {
      renderRejected(file.name, "File exceeds 50 MB limit.");
      continue;
    }
    enqueue(file);
    accepted++;
  }

  if (accepted > 0) controls.removeAttribute("hidden");
}

function enqueue(file) {
  const id = idCounter++;
  queue.set(id, { filename: file.name, state: "pending" });
  updateProgress();
  renderPending(id, file.name);

  const reader = new FileReader();
  reader.onload = (e) => {
    queue.get(id).state = "processing";
    worker.postMessage(
      { type: "process", id, filename: file.name, buffer: e.target.result },
      [e.target.result]
    );
  };
  reader.readAsArrayBuffer(file);
}

// ── Progress ──────────────────────────────────────────────────────────────────
function updateProgress() {
  const total = queue.size;
  const msg = doneCount < total
    ? `Processing ${doneCount} / ${total}…`
    : `${total} file${total !== 1 ? "s" : ""} processed`;
  progressEl.textContent = msg;
  srStatus.textContent = msg;
}

// ── Rendering ─────────────────────────────────────────────────────────────────
function renderPending(id, filename) {
  const row = document.createElement("div");
  row.className = "result-row pending";
  row.id = `row-${id}`;
  row.innerHTML = `
    <div class="row-header">
      <span class="filename">${esc(filename)}</span>
      <span class="row-status">Processing…</span>
    </div>`;
  resultsEl.prepend(row);
}

function renderResult(id, filename, { tags, buffer, originalSize, durationMs }) {
  const row = document.getElementById(`row-${id}`);
  if (!row) return;
  row.className = "result-row done";

  const cleanName = `clean_${filename}`;
  const url = URL.createObjectURL(new Blob([buffer]));
  const savedPct = originalSize > 0
    ? Math.round((1 - buffer.byteLength / originalSize) * 100)
    : 0;
  const sizeLabel = `${formatBytes(originalSize)} → ${formatBytes(buffer.byteLength)}`
    + (savedPct > 0 ? ` (−${savedPct}%)` : "");

  const durationHtml =
    durationMs != null
      ? `<div class="duration-info${durationMs > 500 ? " slow" : ""}">Processed in ${durationMs} ms${
          durationMs > 500
            ? " — slower than the usual target; large files or a cold WASM load can cause this."
            : ""
        }</div>`
      : "";

  row.innerHTML = `
    <div class="row-header">
      <span class="filename">${esc(filename)}</span>
      <span class="row-status done">Done</span>
    </div>
    <div class="size-info">${esc(sizeLabel)}</div>
    ${durationHtml}
    ${renderTags(tags)}
    <a class="download-btn" href="${url}" download="${esc(cleanName)}">
      Download ${esc(cleanName)}
    </a>`;
}

function renderError(id, filename, message) {
  const row = document.getElementById(`row-${id}`);
  if (!row) return;
  row.className = "result-row error";
  row.innerHTML = `
    <div class="row-header">
      <span class="filename">${esc(filename)}</span>
      <span class="row-status error">Error</span>
    </div>
    <p class="size-info">${esc(message)}</p>`;
}

function renderRejected(filename, reason) {
  const row = document.createElement("div");
  row.className = "result-row error";
  row.innerHTML = `
    <div class="row-header">
      <span class="filename">${esc(filename)}</span>
      <span class="row-status error">Rejected</span>
    </div>
    <p class="size-info">${esc(reason)}</p>`;
  resultsEl.prepend(row);
  controls.removeAttribute("hidden");
}

function showGlobalError(msg) {
  const el = document.createElement("p");
  el.className = "size-info";
  el.style.color = "var(--error)";
  el.textContent = msg;
  controls.appendChild(el);
}

// ── Tag rendering ─────────────────────────────────────────────────────────────
const THUMBNAIL_TAG_IDS = new Set([0x0201, 0x0202, 0x0213]); // JPEGInterchangeFormat, length, YCbCrPositioning

function renderTags(tags) {
  if (!tags || tags.length === 0) {
    return `<p class="no-meta">No metadata found</p>`;
  }

  const hasThumbnail = tags.some((t) => THUMBNAIL_TAG_IDS.has(t.id));
  const thumbnailNote = hasThumbnail
    ? `<p class="size-info" style="color:var(--muted)">⚠ Embedded thumbnail detected — removed from output</p>`
    : "";

  const byCategory = {};
  for (const tag of tags) {
    if (!byCategory[tag.category]) byCategory[tag.category] = [];
    byCategory[tag.category].push(tag);
  }

  const sections = Object.entries(byCategory)
    .map(([cat, items]) => `
      <div class="tag-category">
        <h4>${esc(cat)}</h4>
        <ul class="tag-list">
          ${items
            .map(
              (t) =>
                `<li><span class="tag-name">${esc(t.name)}</span>
                     <span class="tag-value">${esc(t.value)}</span></li>`
            )
            .join("")}
        </ul>
      </div>`)
    .join("");

  return thumbnailNote + `<details><summary>Metadata found (${tags.length} tag${tags.length !== 1 ? "s" : ""})</summary>${sections}</details>`;
}

// ── Download all as ZIP ───────────────────────────────────────────────────────
downloadAllBtn.addEventListener("click", () => {
  if (completedBuffers.length === 0) return;
  const files = {};
  for (const { filename, buffer } of completedBuffers) {
    files[filename] = new Uint8Array(buffer);
  }
  const zipped = zipSync(files);
  triggerDownload(new Blob([zipped], { type: "application/zip" }), "clean_images.zip");
});

function triggerDownload(blob, filename) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = filename;
  a.click();
  URL.revokeObjectURL(a.href);
}

// ── Modal ─────────────────────────────────────────────────────────────────────
howBtn.addEventListener("click", () => modal.showModal());
modalClose.addEventListener("click", () => modal.close());

// ── Helpers ───────────────────────────────────────────────────────────────────
function esc(str) {
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function formatBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 ** 2) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 ** 2).toFixed(1)} MB`;
}
