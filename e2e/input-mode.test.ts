import { test, expect } from "@microsoft/tui-test";
import path from "node:path";
import process from "node:process";

const catalystBin = path.resolve(process.cwd(), "..", "target", "release", "catalyst");

test.use({
  program: { file: catalystBin },
  columns: 100,
  rows: 30,
});

test.describe("Input Mode", () => {

  test("switches to INSERT mode on 'i' key", async ({ terminal }) => {
    await expect(terminal.getByText("NORMAL")).toBeVisible();

    terminal.write("i");

    await expect(terminal.getByText("INSERT")).toBeVisible();
  });

  test("returns to NORMAL mode on Escape", async ({ terminal }) => {
    terminal.write("i");
    await expect(terminal.getByText("INSERT")).toBeVisible();

    terminal.keyEscape();
    await expect(terminal.getByText("NORMAL")).toBeVisible();
  });

  test("typed text appears in INSERT mode", async ({ terminal }) => {
    terminal.write("i");
    await expect(terminal.getByText("INSERT")).toBeVisible();

    terminal.write("hello catalyst");

    await expect(terminal.getByText("hello catalyst")).toBeVisible();
  });

  test("snapshot of INSERT mode", async ({ terminal }) => {
    terminal.write("i");
    terminal.write("typing a message");
    await expect(terminal.getByText("typing a message")).toBeVisible();

    await expect(terminal).toMatchSnapshot();
  });
});
