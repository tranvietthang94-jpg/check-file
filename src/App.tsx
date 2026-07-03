import { useEffect } from "react";
import {
  cancelCopy,
  listDisks,
  onCopyCancelled,
  onCopyComplete,
  onCopyProgress,
  onCopyScan,
  onDisksChanged,
  startCopy,
} from "./lib/tauri";
import { useDisksStore } from "./state/disksStore";
import { useTransfersStore } from "./state/transfersStore";
import { DisksPanel } from "./components/DisksPanel";
import { EndpointList } from "./components/EndpointList";
import { TransfersPanel } from "./components/TransfersPanel";
import type { DiskInfo, Endpoint } from "./types/disk";
import "./App.css";

function endpointLabel(endpoint: Endpoint, disks: DiskInfo[]) {
  const disk = disks.find((d) => d.id === endpoint.diskId);
  return endpoint.label || disk?.name || endpoint.diskId;
}

function App() {
  const disks = useDisksStore((s) => s.disks);
  const sources = useDisksStore((s) => s.sources);
  const destinations = useDisksStore((s) => s.destinations);
  const setDisks = useDisksStore((s) => s.setDisks);
  const removeSource = useDisksStore((s) => s.removeSource);
  const removeDestination = useDisksStore((s) => s.removeDestination);
  const setSourceLabel = useDisksStore((s) => s.setSourceLabel);
  const setDestinationLabel = useDisksStore((s) => s.setDestinationLabel);
  const setSourcePath = useDisksStore((s) => s.setSourcePath);
  const setDestinationPath = useDisksStore((s) => s.setDestinationPath);

  const jobs = useTransfersStore((s) => s.jobs);
  const addJob = useTransfersStore((s) => s.addJob);
  const applyScan = useTransfersStore((s) => s.applyScan);
  const applyProgress = useTransfersStore((s) => s.applyProgress);
  const applyComplete = useTransfersStore((s) => s.applyComplete);
  const applyCancelled = useTransfersStore((s) => s.applyCancelled);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listDisks().then(setDisks).catch(console.error);
    onDisksChanged(setDisks).then((fn) => {
      unlisten = fn;
    });

    return () => unlisten?.();
  }, [setDisks]);

  useEffect(() => {
    const unlistenPromises = [
      onCopyScan(applyScan),
      onCopyProgress(applyProgress),
      onCopyComplete(applyComplete),
      onCopyCancelled(applyCancelled),
    ];

    return () => {
      unlistenPromises.forEach((p) => p.then((fn) => fn()));
    };
  }, [applyScan, applyProgress, applyComplete, applyCancelled]);

  async function handleStart(source: Endpoint, destination: Endpoint) {
    const jobId = await startCopy(source.path, destination.path);
    addJob({
      id: jobId,
      sourceDiskId: source.diskId,
      destinationDiskId: destination.diskId,
      sourceLabel: endpointLabel(source, disks),
      destinationLabel: endpointLabel(destination, disks),
      sourcePath: source.path,
      destinationPath: destination.path,
      status: "scanning",
      currentFile: "",
      bytesCopied: 0,
      totalBytes: 0,
      filesCopied: 0,
      totalFiles: 0,
      bytesPerSec: 0,
      failedFiles: [],
    });
  }

  function handleCancel(jobId: string) {
    cancelCopy(jobId).catch(console.error);
  }

  return (
    <main className="min-h-screen bg-neutral-950 p-6 text-neutral-100">
      <h1 className="mb-6 text-xl font-semibold">OffloadKit</h1>
      <div className="grid grid-cols-1 gap-6 md:grid-cols-4">
        <DisksPanel />
        <EndpointList
          title="Sources"
          endpoints={sources}
          disks={disks}
          onRemove={removeSource}
          onLabelChange={setSourceLabel}
          onPathChange={setSourcePath}
        />
        <EndpointList
          title="Destinations"
          endpoints={destinations}
          disks={disks}
          onRemove={removeDestination}
          onLabelChange={setDestinationLabel}
          onPathChange={setDestinationPath}
        />
        <TransfersPanel
          sources={sources}
          destinations={destinations}
          disks={disks}
          jobs={Object.values(jobs)}
          onStart={handleStart}
          onCancel={handleCancel}
        />
      </div>
    </main>
  );
}

export default App;
