import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?referenceFixture=disks");
});

test("global shortcuts switch views and open the matching settings tab", async ({ page }) => {
  await page.keyboard.press("Control+t");
  await expect(page.getByRole("button", { name: "Truyền tải" })).toHaveAttribute("aria-pressed", "true");

  await page.keyboard.press("Control+d");
  await expect(page.getByTestId("disks-shell")).toBeVisible();

  await page.keyboard.press("Control+2");
  await expect(page.getByText("Thuật toán mã băm")).toBeVisible();
});

test("Control+L opens transfer logs", async ({ page }) => {
  await page.keyboard.press("Control+l");
  await expect(page.getByText("Nhật ký truyền tải", { exact: true })).toBeVisible();
});
