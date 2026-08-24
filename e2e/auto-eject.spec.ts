import { expect, test } from "@playwright/test";

async function enableAutoEject(page: import("@playwright/test").Page) {
  await page.addInitScript(() => {
    localStorage.setItem(
      "offloadkit-settings-v1",
      JSON.stringify({ state: { autoEjectEnabled: true }, version: 0 }),
    );
  });
}

test("auto Eject runs once after every expected group job completes cleanly", async ({ page }) => {
  const mounts: string[] = [];
  await enableAutoEject(page);
  await page.exposeFunction("recordAutoEject", (mount: string) => mounts.push(mount));
  await page.addInitScript(() => {
    (window as Window & { __OFFLOADKIT_TEST_EJECT__?: (mount: string) => void }).__OFFLOADKIT_TEST_EJECT__ =
      (mount) => void (window as Window & { recordAutoEject?: (value: string) => void }).recordAutoEject?.(mount);
  });

  await page.goto("/?referenceFixture=autoEject");
  await expect.poll(() => mounts).toEqual(["D:\\"]);
});

test("auto Eject waits when a cascade job has not been created yet", async ({ page }) => {
  const mounts: string[] = [];
  await enableAutoEject(page);
  await page.exposeFunction("recordAutoEject", (mount: string) => mounts.push(mount));
  await page.addInitScript(() => {
    (window as Window & { __OFFLOADKIT_TEST_EJECT__?: (mount: string) => void }).__OFFLOADKIT_TEST_EJECT__ =
      (mount) => void (window as Window & { recordAutoEject?: (value: string) => void }).recordAutoEject?.(mount);
  });

  await page.goto("/?referenceFixture=autoEjectPending");
  await page.getByRole("button", { name: "Truyền tải" }).waitFor();
  await page.waitForTimeout(300);
  expect(mounts).toEqual([]);
});
