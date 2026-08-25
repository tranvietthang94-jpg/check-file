import { expect, test, type Page } from "@playwright/test";

const macUserAgent =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7) AppleWebKit/537.36 Chrome/128 Safari/537.36";

async function openGeneralPreferences(page: Page) {
  await page.goto("/");
  await page.keyboard.press("Control+,");
  await expect(page.getByLabel("Cài đặt")).toBeVisible();
}

test("Windows shows only the Explorer integration control", async ({ page }) => {
  await openGeneralPreferences(page);

  await expect(page.getByRole("heading", { name: "Windows Explorer Integration" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "macOS Finder Integration" })).toHaveCount(0);
});

test.describe("macOS Finder preference", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript((userAgent) => {
      Object.defineProperty(navigator, "userAgent", { configurable: true, get: () => userAgent });
      const disabled = {
        supported: true,
        installed: false,
        healthy: false,
        misplacedApp: false,
        executablePath: "/Applications/OffloadKit.app/Contents/MacOS/offloadkit",
        expectedWorkflows: 4,
        installedWorkflows: 0,
        matchingWorkflows: 0,
        problems: [],
        message: null,
      };
      const enabled = {
        ...disabled,
        installed: true,
        healthy: true,
        installedWorkflows: 4,
        matchingWorkflows: 4,
      };
      let status = disabled;
      (window as Window & {
        __OFFLOADKIT_TEST_FINDER_INTEGRATION__?: {
          status: () => Promise<typeof disabled>;
          install: () => Promise<typeof disabled>;
          uninstall: () => Promise<typeof disabled>;
        };
      }).__OFFLOADKIT_TEST_FINDER_INTEGRATION__ = {
        status: async () => status,
        install: async () => {
          await new Promise((resolve) => setTimeout(resolve, 100));
          status = enabled;
          return status;
        },
        uninstall: async () => {
          await new Promise((resolve) => setTimeout(resolve, 100));
          status = disabled;
          return status;
        },
      };
    }, macUserAgent);
  });

  test("shows only Finder and reflects install and uninstall read-back", async ({ page }) => {
    await openGeneralPreferences(page);

    await expect(page.getByRole("heading", { name: "macOS Finder Integration" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Windows Explorer Integration" })).toHaveCount(0);
    const toggle = page.getByRole("checkbox", { name: "Bật Finder Quick Actions" });
    await expect(toggle).not.toBeChecked();

    await toggle.check();
    await expect(toggle).toBeDisabled();
    await expect(toggle).toBeChecked();
    await expect(page.getByRole("status")).toContainText("Đã bật Finder Quick Actions");

    await toggle.uncheck();
    await expect(toggle).toBeDisabled();
    await expect(toggle).not.toBeChecked();
    await expect(page.getByRole("status")).toContainText("Đã tắt Finder Quick Actions");
  });

  test("shows Applications guidance from a misplaced status", async ({ page }) => {
    await page.addInitScript(() => {
      const testWindow = window as Window & {
        __OFFLOADKIT_TEST_FINDER_INTEGRATION__?: {
          status: () => Promise<Record<string, unknown>>;
        };
      };
      const original = testWindow.__OFFLOADKIT_TEST_FINDER_INTEGRATION__!;
      original.status = async () => ({
        supported: true,
        installed: true,
        healthy: false,
        misplacedApp: true,
        executablePath: "/Users/operator/Downloads/OffloadKit.app/Contents/MacOS/offloadkit",
        expectedWorkflows: 4,
        installedWorkflows: 4,
        matchingWorkflows: 0,
        problems: ["OffloadKit must run from /Applications/OffloadKit.app"],
        message: "Move OffloadKit.app to /Applications before enabling Finder Quick Actions.",
      });
    });
    await openGeneralPreferences(page);

    await expect(page.getByRole("alert")).toContainText("/Applications");
    await expect(page.getByRole("checkbox", { name: "Bật Finder Quick Actions" })).not.toBeChecked();
  });

  test("reports an install error and restores the disabled toggle", async ({ page }) => {
    await page.addInitScript(() => {
      const testWindow = window as Window & {
        __OFFLOADKIT_TEST_FINDER_INTEGRATION__?: {
          install: () => Promise<Record<string, unknown>>;
        };
      };
      testWindow.__OFFLOADKIT_TEST_FINDER_INTEGRATION__!.install = async () => {
        throw new Error("macOS refused the Services update");
      };
    });
    await openGeneralPreferences(page);
    const toggle = page.getByRole("checkbox", { name: "Bật Finder Quick Actions" });

    await toggle.click();

    await expect(page.getByRole("alert")).toContainText("macOS refused the Services update");
    await expect(toggle).not.toBeChecked();
    await expect(toggle).toBeEnabled();
  });
});
