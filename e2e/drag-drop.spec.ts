import { expect, test } from "@playwright/test";

async function dispatchDiskDrop(page: import("@playwright/test").Page, diskId: string, targetTestId: string) {
  await page.getByTestId(targetTestId).evaluate((target, id) => {
    const transfer = new DataTransfer();
    transfer.setData("application/x-offloadkit-disk-id", id);
    target.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: transfer }));
    target.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: transfer }));
  }, diskId);
}

test.beforeEach(async ({ page }) => {
  await page.goto("/?referenceFixture=disks");
});

test("dropping an available disk into Sources assigns it once and removes it from the grid", async ({ page }) => {
  await dispatchDiskDrop(page, "D:", "sources-drop-zone");
  await expect(page.getByTestId("source-endpoint-card")).toHaveCount(1);
  await expect(page.getByTestId("available-disk-card").filter({ hasText: "KHANH VAN" })).toHaveCount(0);

  await dispatchDiskDrop(page, "D:", "sources-drop-zone");
  await expect(page.getByTestId("source-endpoint-card")).toHaveCount(1);
});

test("unknown disk payload cannot mutate Sources", async ({ page }) => {
  await dispatchDiskDrop(page, "unknown-disk", "sources-drop-zone");
  await expect(page.getByTestId("source-endpoint-card")).toHaveCount(0);
  await expect(page.getByTestId("available-disk-card")).toHaveCount(7);
});
