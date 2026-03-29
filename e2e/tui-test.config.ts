import { defineConfig } from "@microsoft/tui-test";
import path from "node:path";

const catalystBin = path.resolve(import.meta.dirname, "..", "target", "release", "catalyst");

export default defineConfig({
  retries: 2,
  trace: false,
});
