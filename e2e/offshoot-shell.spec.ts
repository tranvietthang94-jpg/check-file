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

  const diskGrid = page.getByTestId("available-disk-grid");
  const diskCards = page.getByTestId("available-disk-card");
  await expect(diskCards).toHaveCount(7);
  await expect(page.getByTestId("available-disk-card").filter({ hasText: "DATA" })).toHaveCount(0);

  const [gridBox, firstCardBox, secondCardBox, fourthCardBox] = await Promise.all([
    diskGrid.boundingBox(),
    diskCards.nth(0).boundingBox(),
    diskCards.nth(1).boundingBox(),
    diskCards.nth(3).boundingBox(),
  ]);
  expect(gridBox).not.toBeNull();
  expect(firstCardBox).not.toBeNull();
  expect(secondCardBox).not.toBeNull();
  expect(fourthCardBox).not.toBeNull();
  expect(firstCardBox!.width).toBeGreaterThanOrEqual(140);
  expect(firstCardBox!.height).toBeGreaterThanOrEqual(145);
  expect(Math.abs(firstCardBox!.y - secondCardBox!.y)).toBeLessThan(2);
  expect(fourthCardBox!.y).toBeGreaterThan(firstCardBox!.y + firstCardBox!.height);
  expect(gridBox!.width).toBeGreaterThan(firstCardBox!.width * 3);
});
