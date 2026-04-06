// Web Worker — initialises WASM once, then processes files on demand.
// Protocol:
//   Incoming: { type: "process", id, filename, buffer }   (buffer transferred)
//   Outgoing: { type: "result",  id, filename, buffer, tags }  (buffer transferred)
//             { type: "error",   id, filename, message }

let wasm = null;

async function init() {
  const mod = await import("@wasm-pkg/exif_stripper_wasm.js");
  await mod.default(); // initialise the WASM module
  wasm = mod;
}

const ready = init();

self.onmessage = async ({ data }) => {
  await ready;

  const { type, id, filename, buffer } = data;
  if (type !== "process") return;

  const t0 = performance.now();
  try {
    const input = new Uint8Array(buffer);
    const ext = filename.split(".").pop().toLowerCase();

    // Read tags before stripping
    let tags = [];
    try {
      const json = wasm.read_exif_tags(input);
      tags = JSON.parse(json);
    } catch (_) {
      // non-fatal — proceed without tags
    }

    let cleaned;
    if (ext === "png") {
      cleaned = wasm.strip_png(input);
    } else {
      cleaned = wasm.strip_jpeg(input);
    }

    const durationMs = Math.round(performance.now() - t0);
    self.postMessage(
      {
        type: "result",
        id,
        filename,
        buffer: cleaned.buffer,
        tags,
        originalSize: input.byteLength,
        durationMs,
      },
      [cleaned.buffer]
    );
  } catch (err) {
    self.postMessage({
      type: "error",
      id,
      filename,
      message: err instanceof Error ? err.message : String(err),
    });
  }
};
