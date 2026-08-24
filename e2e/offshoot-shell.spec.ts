import { expect, test } from "@playwright/test";

test("reference fixture renders OffShoot-style three-column shell", async ({ page }) => {
  await page.goto("/?referenceFixture=disks");

  const shell = page.getByTestId("disks-shell");
  const sources = page.getByTestId("sources-column");
  const disks = page.getByTestId("disks-column");
  const destinations = page.getByTestId("destinations-column");

  await expect(shell).toBeVisible();
  await expect(sources).toBeVisible();
  await expect(disks).toBeVisible();
  await expect(destinations).toBeVisible();

  const [shellBox, sourceBox, disksBox, destinationBox] = await Promise.all([
    shell.boundingBox(),
    sources.boundingBox(),
    disks.boundingBox(),
    destinations.boundingBox(),
  ]);

  expect(shellBox).not.toBeNull();
  expect(sourceBox).not.toBeNull();
  expect(disksBox).not.toBeNull();
  expect(destinationBox).not.toBeNull();
  expect(disksBox!.width).toBeGreaterThan(sourceBox!.width * 2);
  expect(disksBox!.width).toBeGreaterThan(destinationBox!.width * 2);
  expect(Math.abs(sourceBox!.width - destinationBox!.width)).toBeLessThan(8);

  await expect(page.getByText("KHANH VAN", { exact: false })).toBeVisible();
  await expect(page.getByText("DATA", { exact: false })).toBeVisible();
});
