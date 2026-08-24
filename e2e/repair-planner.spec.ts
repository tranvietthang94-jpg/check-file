import { expect, test } from "@playwright/test";

test("repair planner opens from an MHL problem and requires a verified candidate", async ({ page }) => {
  await page.goto("/?referenceFixture=transfers");
  await page.getByRole("button", { name: "Truyền tải" }).waitFor();

  await page.evaluate(() => {
    const store = (window as Window & { __OFFLOADKIT_MHL_TEST__?: (value: unknown) => void }).__OFFLOADKIT_MHL_TEST__;
    store?.({
      mhlPath: "D:\\Offload\\transfer.mhl",
      results: [{ relativePath: "clip.mov", status: "mismatch" }],
    });
  });

  await expect(page.getByRole("button", { name: "Lập kế hoạch sửa" })).toBeVisible();
});
