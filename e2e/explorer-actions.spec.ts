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
