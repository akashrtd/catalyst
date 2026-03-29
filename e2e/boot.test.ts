import { test, expect } from "@microsoft/tui-test";
import path from "node:path";
import process from "node:process";

const catalystBin = path.resolve(process.cwd(), "..", "target", "release", "catalyst");

test.use({
  program: { file: catalystBin },
  columns: 100,
  rows: 30,
});

test.describe("Catalyst TUI", () => {

  test("boots and renders header", async ({ terminal }) => {
    await expect(terminal.getByText("CATALYST")).toBeVisible();
  });

  test("renders default model in header", async ({ terminal }) => {
    await expect(terminal.getByText("claude-sonnet-4-20250514", { strict: false })).toBeVisible();
  });

  test("renders input area in NORMAL mode", async ({ terminal }) => {
    await expect(terminal.getByText("NORMAL")).toBeVisible();
  });

  test("renders footer shortcuts", async ({ terminal }) => {
    await expect(terminal.getByText("nsert")).toBeVisible();
    await expect(terminal.getByText("normal")).toBeVisible();
    await expect(terminal.getByText("send")).toBeVisible();
    await expect(terminal.getByText("cmds")).toBeVisible();
  });

  test("starts in NORMAL mode indicator", async ({ terminal }) => {
    await expect(terminal.getByText("NORMAL")).toBeVisible();
  });

  test("take initial snapshot", async ({ terminal }) => {
    await expect(terminal).toMatchSnapshot();
  });
});
