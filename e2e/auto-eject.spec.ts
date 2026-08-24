import { expect, test } from "@playwright/test";

test("auto Eject only arms for a completely successful removable-source group", async ({ page }) => {
  await page.goto("/?referenceFixture=autoEject");
  await page.getByRole("button", { name: "Truyền tải" }).waitFor();

  await expect(page.getByText("KHANH VAN → DATA", { exact: false })).toBeVisible();
  await expect(page.getByText("DATA", { exact: true })).toBeVisible();
});
