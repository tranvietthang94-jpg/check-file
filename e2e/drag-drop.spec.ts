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

test("dropping an available disk into Destinations appends it and removes it from the grid", async ({ page }) => {
  await dispatchDiskDrop(page, "D:", "destinations-drop-zone");
  await expect(page.getByTestId("destination-endpoint-card")).toHaveCount(2);
  await expect(page.getByTestId("destination-endpoint-card").last()).toContainText("KHANH VAN");
  await expect(page.getByTestId("available-disk-card").filter({ hasText: "KHANH VAN" })).toHaveCount(0);
});

test("dragging an assigned source back to the disk grid removes its assignment", async ({ page }) => {
  await dispatchDiskDrop(page, "D:", "sources-drop-zone");
  await expect(page.getByTestId("source-endpoint-card")).toHaveCount(1);
  await page.getByTestId("source-endpoint-card").evaluate((target) => {
    const transfer = new DataTransfer();
    transfer.setData("application/x-offloadkit-endpoint-remove", "D:");
    target.dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: transfer }));
    const grid = document.querySelector('[data-testid="available-disk-grid"]');
    grid?.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: transfer }));
    grid?.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: transfer }));
  });
  await expect(page.getByTestId("source-endpoint-card")).toHaveCount(0);
  await expect(page.getByTestId("available-disk-card").filter({ hasText: "KHANH VAN" })).toHaveCount(1);
});

async function dispatchDestinationReorder(
  page: import("@playwright/test").Page,
  fromDiskId: string,
  targetIndex: number,
  clientY: number,
) {
  await page.getByTestId("destination-endpoint-card").nth(targetIndex).evaluate(
    (target, payload) => {
      const transfer = new DataTransfer();
      transfer.setData("application/x-offloadkit-endpoint-reorder", payload.fromDiskId);
      target.dispatchEvent(
        new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: transfer, clientY: payload.clientY }),
      );
      target.dispatchEvent(
        new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: transfer, clientY: payload.clientY }),
      );
    },
    { fromDiskId, clientY },
  );
}

test("destination reorder shows an insertion marker and moves after the target midpoint", async ({ page }) => {
  await dispatchDiskDrop(page, "D:", "destinations-drop-zone");
  await dispatchDiskDrop(page, "G:", "destinations-drop-zone");
  const cards = page.getByTestId("destination-endpoint-card");
  await expect(cards).toHaveCount(3);

  const targetBox = await cards.nth(1).boundingBox();
  expect(targetBox).not.toBeNull();
  await cards.nth(1).evaluate((target, payload) => {
    const transfer = new DataTransfer();
    transfer.setData("application/x-offloadkit-endpoint-reorder", "G:");
    target.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: transfer, clientY: payload }));
  }, targetBox!.y + targetBox!.height - 2);
  await expect(page.getByTestId("destination-insertion-after")).toBeVisible();

  await dispatchDestinationReorder(page, "G:", 1, targetBox!.y + targetBox!.height - 2);
  await expect(cards.nth(0)).toContainText("DATA");
  await expect(cards.nth(1)).toContainText("KHANH VAN");
  await expect(cards.nth(2)).toContainText("Local Disk I");
});
