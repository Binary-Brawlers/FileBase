import { readFile } from "node:fs/promises";

const [entrypoint, declarations, implementation] = await Promise.all([
  readFile(new URL("../dist/index.js", import.meta.url), "utf8"),
  readFile(new URL("../dist/index.d.ts", import.meta.url), "utf8"),
  readFile(new URL("../dist/useUpload.js", import.meta.url), "utf8").catch(
    () => "",
  ),
]);

if (
  !entrypoint.includes("useUpload") ||
  !declarations.includes("useUpload") ||
  !implementation.includes("useUpload")
) {
  throw new Error(
    "The Vue SDK dist is stale and does not export useUpload. Generate dist before publishing.",
  );
}
