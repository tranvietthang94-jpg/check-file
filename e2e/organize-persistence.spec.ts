import { expect, test } from "@playwright/test";

test("organize settings persist after reload", async ({ page }) => {
  await page.goto("/?referenceFixture=disks");
  await page.getByTestId("disks-shell").waitFor();
  await page.keyboard.press("Control+3");

  const flatten = page.getByRole("checkbox", { name: "Làm phẳng (bỏ thư mục con gốc)" });
  await flatten.check();
  await page.reload();
  await page.getByTestId("disks-shell").waitFor();
  await page.keyboard.press("Control+3");

  await expect(flatten).toBeChecked();
});
