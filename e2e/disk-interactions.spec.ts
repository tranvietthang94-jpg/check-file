import { expect, test } from "@playwright/test";

async function openFirstDiskLabelEditor(page: import("@playwright/test").Page) {
  const card = page.getByTestId("available-disk-card").first();
  await card.getByRole("button", { name: "KHANH VAN", exact: true }).click();
  return card.getByPlaceholder("Nhãn…");
}

test.beforeEach(async ({ page }) => {
  await page.goto("/?referenceFixture=disks");
});

test("Escape cancels an inline disk label edit without assigning the disk", async ({ page }) => {
  const input = await openFirstDiskLabelEditor(page);
  await input.fill("CAM A");
  await input.press("Escape");

  await expect(input).toBeHidden();
  await expect(page.getByTestId("source-endpoint-card")).toHaveCount(0);
  await expect(page.getByTestId("available-disk-card").first()).toContainText("KHANH VAN");
  await expect(page.getByTestId("available-disk-card").first().getByRole("button", { name: "KHANH VAN", exact: true })).toBeFocused();
});

test("Escape cancels an endpoint label edit and restores the previous label", async ({ page }) => {
  const source = page.getByTestId("source-endpoint-card");
  await page.getByTestId("available-disk-card").first().evaluate((card) => {
    const transfer = new DataTransfer();
    transfer.setData("application/x-offloadkit-disk-id", "D:");
    document.querySelector('[data-testid="sources-drop-zone"]')?.dispatchEvent(
      new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: transfer }),
    );
  });
  await expect(source).toHaveCount(1);
  await source.getByRole("button", { name: "KHANH VAN", exact: true }).click();
  const input = source.getByPlaceholder("Nhãn…");
  await input.fill("TEMP LABEL");
  await input.press("Escape");
  await expect(input).toBeHidden();
  await expect(source).toContainText("KHANH VAN");
});

test("Enter saves an inline disk label and assigns the disk as Source", async ({ page }) => {
  const input = await openFirstDiskLabelEditor(page);
  await input.fill("CAM A");
  await input.press("Enter");

  const source = page.getByTestId("source-endpoint-card");
  await expect(source).toHaveCount(1);
  await expect(source).toContainText("CAM A");
  await expect(page.getByTestId("available-disk-card").filter({ hasText: "KHANH VAN" })).toHaveCount(0);
});
