import { expect, test } from "@playwright/test";

test("auto Source adds removable disks whose name matches the wildcard", async ({ page }) => {
  await page.goto("/?referenceFixture=disks");
  await page.getByTestId("disks-shell").waitFor();
  await page.evaluate(() => {
    localStorage.setItem(
      "offloadkit-settings-v1",
      JSON.stringify({
        state: {
          autoSourceEnabled: true,
          autoSourcePattern: "KHANH*",
          autoEjectEnabled: false,
        },
        version: 0,
      }),
    );
  });
  await page.reload();

  await expect(page.getByText("KHANH VAN", { exact: true })).toBeVisible();
  await expect(page.getByTestId("available-disk-card").filter({ hasText: "KHANH VAN" })).toHaveCount(0);
});
