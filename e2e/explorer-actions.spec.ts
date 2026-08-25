import { expect, test } from "@playwright/test";

test.describe("path-based endpoint store", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("keeps disk endpoints working and adds Source and Destination folders", async ({ page }) => {
    const state = await page.evaluate(async () => {
      const { useDisksStore } = await import("/src/state/disksStore.ts");
      const store = useDisksStore.getState();
      store.setEndpoints([], []);
      store.setDisks([
        {
          id: "F:",
          name: "BACKUP",
          mountPoint: "F:\\",
          totalBytes: 100,
          availableBytes: 50,
          isRemovable: true,
          fileSystem: "NTFS",
        },
      ]);

      store.addSource("F:");
      const sourceResult = store.addSourcePath("F:\\Adobe Premiere Pro Auto-Save");
      const destinationResult = store.addDestinationPath("F:\\BACKUP HXM");
      return {
        sources: useDisksStore.getState().sources,
        destinations: useDisksStore.getState().destinations,
        sourceResult,
        destinationResult,
      };
    });

    expect(state.sources[0]).toMatchObject({ id: "F:", diskId: "F:", path: "F:\\" });
    expect(state.sources[1]).toMatchObject({
      diskId: "F:",
      path: "F:\\Adobe Premiere Pro Auto-Save",
    });
    expect(state.destinations[0]).toMatchObject({ diskId: "F:", path: "F:\\BACKUP HXM" });
    expect(state.sourceResult).toMatchObject({ ok: true, added: true });
    expect(state.destinationResult).toMatchObject({ ok: true, added: true });
  });

  test("deduplicates normalized Windows paths case-insensitively", async ({ page }) => {
    const result = await page.evaluate(async () => {
      const { useDisksStore } = await import("/src/state/disksStore.ts");
      const store = useDisksStore.getState();
      store.setEndpoints([], []);
      store.setDisks([]);

      const first = store.addSourcePath("F:\\Footage\\Day 01\\");
      const duplicate = store.addSourcePath("f:/footage/day 01");
      return { first, duplicate, sources: useDisksStore.getState().sources };
    });

    expect(result.first).toMatchObject({ ok: true, added: true });
    expect(result.duplicate).toMatchObject({ ok: true, added: false });
    expect(result.sources).toHaveLength(1);
  });

  test("keeps unknown paths diskless and exposes no eject action", async ({ page }) => {
    const endpoint = await page.evaluate(async () => {
      const { useDisksStore } = await import("/src/state/disksStore.ts");
      const store = useDisksStore.getState();
      store.setEndpoints([], []);
      store.setDisks([]);
      store.addDestinationPath("Z:\\Offline Destination");
      return useDisksStore.getState().destinations[0];
    });

    expect(endpoint).toMatchObject({ diskId: null, path: "Z:\\Offline Destination" });
    const card = page.getByTestId("destination-endpoint-card");
    await expect(card).toHaveCount(1);
    await expect(card.getByTitle("Tháo")).toHaveCount(0);
  });
});

test.describe("Explorer frontend event bridge", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/?referenceFixture=disks");
    await page.evaluate(async () => {
      const { useDisksStore } = await import("/src/state/disksStore.ts");
      useDisksStore.getState().setEndpoints([], []);
      const testWindow = window as Window & { __OFFLOADKIT_TEST_ACKS__?: string[] };
      testWindow.__OFFLOADKIT_TEST_ACKS__ = [];
      window.addEventListener("offloadkit-test:explorer-ack", ((event: CustomEvent<string>) => {
        testWindow.__OFFLOADKIT_TEST_ACKS__?.push(event.detail);
      }) as EventListener);
    });
  });

  test("set-source event adds Source once, acknowledges, and starts no transfer", async ({ page }) => {
    await page.evaluate(() => {
      const request = {
        id: "source-request-1",
        action: "setSource",
        paths: ["F:\\Adobe Premiere Pro Auto-Save"],
      };
      window.dispatchEvent(
        new CustomEvent("offloadkit-test:explorer-request", { detail: request }),
      );
      window.dispatchEvent(
        new CustomEvent("offloadkit-test:explorer-request", { detail: request }),
      );
    });

    await expect(page.getByTestId("source-endpoint-card")).toHaveCount(1);
    await expect(page.getByTestId("source-endpoint-card")).toContainText(
      "F:\\Adobe Premiere Pro Auto-Save",
    );
    await expect.poll(() => page.evaluate(() =>
      (window as Window & { __OFFLOADKIT_TEST_ACKS__?: string[] }).__OFFLOADKIT_TEST_ACKS__,
    )).toEqual(["source-request-1"]);
    const jobCount = await page.evaluate(async () => {
      const { useTransfersStore } = await import("/src/state/transfersStore.ts");
      return Object.keys(useTransfersStore.getState().jobs).length;
    });
    expect(jobCount).toBe(0);
  });

  test("set-destination event adds Destination", async ({ page }) => {
    await page.evaluate(() => {
      window.dispatchEvent(
        new CustomEvent("offloadkit-test:explorer-request", {
          detail: {
            id: "destination-request-1",
            action: "setDestination",
            paths: ["F:\\BACKUP HXM"],
          },
        }),
      );
    });

    await expect(page.getByTestId("destination-endpoint-card")).toHaveCount(1);
    await expect(page.getByTestId("destination-endpoint-card")).toContainText("F:\\BACKUP HXM");
  });

  test("invalid activation displays a clear error", async ({ page }) => {
    await page.evaluate(() => {
      window.dispatchEvent(
        new CustomEvent("offloadkit-test:explorer-error", {
          detail: { id: "invalid-request-1", message: "Explorer path does not exist" },
        }),
      );
    });

    await expect(page.getByRole("alert")).toContainText("Explorer path does not exist");
  });

  test("request received before hydration waits until the store is ready", async ({ page }) => {
    const beforeReady = await page.evaluate(async () => {
      const { useExplorerActionStore } = await import("/src/state/explorerActionStore.ts");
      const { useDisksStore } = await import("/src/state/disksStore.ts");
      useExplorerActionStore.setState({ ready: false });
      window.dispatchEvent(
        new CustomEvent("offloadkit-test:explorer-request", {
          detail: {
            id: "startup-request-1",
            action: "setSource",
            paths: ["F:\\Startup Source"],
          },
        }),
      );
      await new Promise((resolve) => setTimeout(resolve, 0));
      return useDisksStore.getState().sources.length;
    });
    expect(beforeReady).toBe(0);

    await page.evaluate(async () => {
      const { useExplorerActionStore } = await import("/src/state/explorerActionStore.ts");
      await useExplorerActionStore.getState().markReady();
    });

    await expect(page.getByTestId("source-endpoint-card")).toContainText("F:\\Startup Source");
  });
});
