import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?referenceFixture=transfers");
});

test("transfer fixture renders queued copying completed and failed job states", async ({ page }) => {
  await expect(page.getByRole("heading", { name: "Truyền tải" })).toBeVisible();
  await expect(page.getByTestId("transfer-job-row")).toHaveCount(4);
  await expect(page.getByTestId("transfer-job-row").filter({ hasText: "Đang chờ" })).toHaveCount(1);
  await expect(page.getByTestId("transfer-job-row").filter({ hasText: "Đang sao chép" })).toHaveCount(1);
  await expect(page.getByTestId("transfer-job-row").filter({ hasText: "Hoàn tất" })).toHaveCount(2);
  await expect(page.getByText("1 tệp thất bại", { exact: false })).toBeVisible();
  await expect(page.getByText("MB/s", { exact: false })).toBeVisible();
});

test("copying job exposes progress and cancel while failed completion exposes resume", async ({ page }) => {
  const copying = page.getByTestId("transfer-job-row").filter({ hasText: "clip_004.mov" });
  await expect(copying.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "50");
  await expect(copying.getByRole("button", { name: "Hủy" })).toBeVisible();

  const failed = page.getByTestId("transfer-job-row").filter({ hasText: "1 tệp thất bại" });
  await expect(failed.getByRole("button", { name: "Tiếp tục" })).toBeVisible();
});

test("add transfers bar shows count mode and move eligibility", async ({ page }) => {
  await page.goto("/?referenceFixture=disks");
  const firstDisk = page.getByTestId("available-disk-card").first();
  await firstDisk.evaluate(() => {
    const transfer = new DataTransfer();
    transfer.setData("application/x-offloadkit-disk-id", "D:");
    document.querySelector('[data-testid="sources-drop-zone"]')?.dispatchEvent(
      new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: transfer }),
    );
  });

  const bar = page.getByTestId("add-transfers-bar");
  await expect(bar).toBeVisible();
  await expect(bar.getByRole("button", { name: "Thêm 1 lượt truyền" })).toBeVisible();
  await expect(bar.getByText("Song song", { exact: true })).toBeVisible();
  await expect(bar.getByText("Nối tiếp", { exact: true })).toBeVisible();
  await expect(bar.getByText("Di chuyển", { exact: true })).toBeVisible();
});
