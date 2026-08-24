import { expect, test } from "@playwright/test";

test("settings persist after reload", async ({ page }) => {
  await page.goto("/?referenceFixture=disks");
  await page.getByTestId("disks-shell").waitFor();
  await page.keyboard.press("Control+,");

  const notifications = page.getByRole("checkbox", { name: "Thông báo trên desktop" });
  await notifications.uncheck();
  await page.reload();
  await page.keyboard.press("Control+,");

  await expect(notifications).not.toBeChecked();
});

test("disk automation settings persist after reload", async ({ page }) => {
  await page.goto("/?referenceFixture=disks");
  await page.getByTestId("disks-shell").waitFor();
  await page.keyboard.press("Control+,");
  await page.getByLabel("Cài đặt").getByRole("button", { name: "Ổ đĩa" }).click();

  await page.getByRole("checkbox", { name: "Tự động thêm ổ khớp mẫu làm Nguồn" }).check();
  await page.getByLabel("Mẫu tên ổ Nguồn").fill("CARD_*");
  await page.getByRole("checkbox", { name: "Tự động tháo ổ Nguồn sau khi truyền thành công" }).check();
  await page.reload();
  await page.keyboard.press("Control+,");
  await page.getByLabel("Cài đặt").getByRole("button", { name: "Ổ đĩa" }).click();

  await expect(page.getByRole("checkbox", { name: "Tự động thêm ổ khớp mẫu làm Nguồn" })).toBeChecked();
  await expect(page.getByLabel("Mẫu tên ổ Nguồn")).toHaveValue("CARD_*");
  await expect(page.getByRole("checkbox", { name: "Tự động tháo ổ Nguồn sau khi truyền thành công" })).toBeChecked();
});
