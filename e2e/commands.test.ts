import { test, expect } from "@microsoft/tui-test";
import path from "node:path";
import process from "node:process";

const catalystBin = path.resolve(process.cwd(), "..", "target", "release", "catalyst");

test.use({
  program: { file: catalystBin },
  columns: 100,
  rows: 30,
});

test.describe("Slash Commands", () => {

  test("/help shows command list", async ({ terminal }) => {
    terminal.write("i");
    await expect(terminal.getByText("INSERT")).toBeVisible();

    terminal.submit("/help");

    await expect(terminal.getByText("Commands")).toBeVisible();
    await expect(terminal.getByText("/help", { strict: false })).toBeVisible();
    await expect(terminal.getByText("/model", { strict: false })).toBeVisible();
    await expect(terminal.getByText("/clear", { strict: false })).toBeVisible();
    await expect(terminal.getByText("/config", { strict: false })).toBeVisible();
    await expect(terminal.getByText("/sessions", { strict: false })).toBeVisible();
    await expect(terminal.getByText("/session", { strict: false })).toBeVisible();
    await expect(terminal.getByText("/exit", { strict: false })).toBeVisible();
  });

  test("/help snapshot", async ({ terminal }) => {
    terminal.write("i");
    terminal.submit("/help");
    await expect(terminal.getByText("Commands")).toBeVisible();

    await expect(terminal).toMatchSnapshot();
  });

  test("/config shows current configuration", async ({ terminal }) => {
    terminal.write("i");
    terminal.submit("/config");

    await expect(terminal.getByText("Model:")).toBeVisible();
    await expect(terminal.getByText("Provider:")).toBeVisible();
    await expect(terminal.getByText("Tokens:")).toBeVisible();
    await expect(terminal.getByText("Cost:")).toBeVisible();
    await expect(terminal.getByText("API Key:")).toBeVisible();
  });

  test("/config snapshot", async ({ terminal }) => {
    terminal.write("i");
    terminal.submit("/config");
    await expect(terminal.getByText("Model:")).toBeVisible();

    await expect(terminal).toMatchSnapshot();
  });

  test("/sessions shows empty state", async ({ terminal }) => {
    terminal.write("i");
    terminal.submit("/sessions");

    await expect(terminal.getByText("No saved sessions")).toBeVisible();
  });

  test("/session new starts new session", async ({ terminal }) => {
    terminal.write("i");
    terminal.submit("/session new");

    await expect(terminal.getByText("New session started")).toBeVisible();
  });

  test("/session resume without id shows usage", async ({ terminal }) => {
    terminal.write("i");
    terminal.submit("/session resume");

    await expect(terminal.getByText("Usage: /session resume")).toBeVisible();
  });

  test("/clear clears conversation", async ({ terminal }) => {
    terminal.write("i");
    terminal.submit("/help");
    await expect(terminal.getByText("Commands")).toBeVisible();

    terminal.submit("/clear");

    await expect(terminal.getByText("Conversation cleared")).toBeVisible();
  });

  test("unknown command shows warning", async ({ terminal }) => {
    terminal.write("i");
    terminal.submit("/unknown");

    await expect(terminal.getByText("Unknown command")).toBeVisible();
  });
});
