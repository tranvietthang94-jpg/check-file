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

test.describe("Finder request parity", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript((userAgent) => {
      Object.defineProperty(navigator, "userAgent", { configurable: true, get: () => userAgent });
    }, macUserAgent);
    await page.goto("/?referenceFixture=disks");
    await page.evaluate(async () => {
      const { useDisksStore } = await import("/src/state/disksStore.ts");
      useDisksStore.getState().setEndpoints([], []);
      (window as Window & { __OFFLOADKIT_TEST_ACKS__?: string[] }).__OFFLOADKIT_TEST_ACKS__ = [];
      window.addEventListener("offloadkit-test:explorer-ack", ((event: CustomEvent<string>) => {
        (window as Window & { __OFFLOADKIT_TEST_ACKS__?: string[] })
          .__OFFLOADKIT_TEST_ACKS__?.push(event.detail);
      }) as EventListener);
    });
  });

  test("Source and Destination use the shared endpoint stores with Finder feedback", async ({ page }) => {
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent("offloadkit-test:explorer-request", {
        detail: {
          id: "finder-source-1",
          action: "setSource",
          paths: ["/Volumes/CARD A/DCIM/A001.mov"],
          sourceSelection: {
            commonRoot: "/Volumes/CARD A/DCIM",
            selectedPaths: ["/Volumes/CARD A/DCIM/A001.mov"],
          },
        },
      }));
      window.dispatchEvent(new CustomEvent("offloadkit-test:explorer-request", {
        detail: {
          id: "finder-destination-1",
          action: "setDestination",
          paths: ["/Volumes/BACKUP"],
        },
      }));
    });

    await expect(page.getByTestId("source-endpoint-card")).toContainText("/Volumes/CARD A/DCIM");
    await expect(page.getByTestId("destination-endpoint-card")).toContainText("/Volumes/BACKUP");
    await expect(page.getByTestId("explorer-feedback")).toContainText("Finder");
  });

  test("Copy acknowledges without mutating the composer", async ({ page }) => {
    const before = await page.evaluate(async () => {
      const { useDisksStore } = await import("/src/state/disksStore.ts");
      useDisksStore.getState().addSourcePath("/Volumes/OLD SOURCE");
      useDisksStore.getState().addDestinationPath("/Volumes/OLD DESTINATION");
      return {
        sources: useDisksStore.getState().sources,
        destinations: useDisksStore.getState().destinations,
      };
    });

    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent("offloadkit-test:explorer-request", {
        detail: {
          id: "finder-copy-1",
          action: "copy",
          paths: ["/Volumes/CARD A/A001.mov", "/Volumes/CARD A/A002.mov"],
        },
      }));
    });

    await expect(page.getByTestId("explorer-feedback")).toContainText("Finder");
    const after = await page.evaluate(async () => {
      const { useDisksStore } = await import("/src/state/disksStore.ts");
      return {
        sources: useDisksStore.getState().sources,
        destinations: useDisksStore.getState().destinations,
      };
    });
    expect(after).toEqual(before);
  });

  test("Paste starts one selected-path copy with both move flags false", async ({ page }) => {
    await page.evaluate(() => {
      const testWindow = window as Window & {
        __OFFLOADKIT_TEST_START_TRANSFER__?: (payload: unknown) => string;
        __OFFLOADKIT_TEST_STARTS__?: unknown[];
      };
      testWindow.__OFFLOADKIT_TEST_STARTS__ = [];
      testWindow.__OFFLOADKIT_TEST_START_TRANSFER__ = (payload) => {
        testWindow.__OFFLOADKIT_TEST_STARTS__?.push(payload);
        return "finder-paste-group";
      };
      window.dispatchEvent(new CustomEvent("offloadkit-test:explorer-request", {
        detail: {
          id: "finder-paste-1",
          action: "paste",
          paths: ["/Volumes/BACKUP"],
          sourceSelection: {
            commonRoot: "/Volumes/CARD A",
            selectedPaths: ["/Volumes/CARD A/DCIM", "/Volumes/CARD A/A001.wav"],
          },
          destinationPath: "/Volumes/BACKUP",
        },
      }));
    });

    await expect(page.getByTestId("explorer-feedback")).toContainText("Finder");
    const starts = await page.evaluate(() =>
      (window as Window & { __OFFLOADKIT_TEST_STARTS__?: unknown[] })
        .__OFFLOADKIT_TEST_STARTS__,
    );
    expect(starts).toEqual([
      expect.objectContaining({
        source: "/Volumes/CARD A",
        selectedPaths: ["/Volumes/CARD A/DCIM", "/Volumes/CARD A/A001.wav"],
        destinations: ["/Volumes/BACKUP"],
        moveAfterTransfer: false,
        moveSameVolume: false,
      }),
    ]);
  });
});
