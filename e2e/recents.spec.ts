import { expect, test } from "@playwright/test";

test("recent Source folders are shown and can be cleared", async ({ page }) => {
  await page.goto("/?referenceFixture=disks");
  await page.getByTestId("disks-shell").waitFor();
  await page.evaluate(() => {
    localStorage.setItem(
      "offloadkit-recents-v1",
      JSON.stringify({ state: { recentSources: ["D:\\DCIM"], recentDestinations: [] }, version: 0 }),
    );
  });
  await page.reload();

  await page.getByTestId("available-disk-card").first().click({ button: "right" });
  await page.getByRole("menuitem", { name: "Thư mục Nguồn" }).hover();
  await expect(page.getByRole("menuitem", { name: "D:\\DCIM" })).toBeVisible();
  await page.getByRole("menuitem", { name: "Xóa thư mục gần đây" }).click();

  await page.getByTestId("available-disk-card").first().click({ button: "right" });
  await page.getByRole("menuitem", { name: "Thư mục Nguồn" }).hover();
  await expect(page.getByRole("menuitem", { name: "D:\\DCIM" })).toHaveCount(0);
});
