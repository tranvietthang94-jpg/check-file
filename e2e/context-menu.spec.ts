import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?referenceFixture=disks");
});

test("disk context menu stays inside the viewport near the bottom-right edge", async ({ page }) => {
  await page.getByTestId("available-disk-card").last().click({ button: "right" });

  const menu = page.getByRole("menu").first();
  await expect(menu).toBeVisible();
  const box = await menu.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.y).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(1440);
  expect(box!.y + box!.height).toBeLessThanOrEqual(900);
});

test("disk context menu supports keyboard selection and restores focus", async ({ page }) => {
  const card = page.getByTestId("available-disk-card").first();
  await card.focus();
  await card.click({ button: "right" });

  const menu = page.getByRole("menu").first();
  await expect(menu).toBeVisible();
  await page.keyboard.press("ArrowDown");
  await expect(page.getByRole("menuitem").first()).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(menu).toBeHidden();
  await expect(card).toBeFocused();
});
