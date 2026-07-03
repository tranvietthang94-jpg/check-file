import { useEffect } from "react";
import { listDisks, onDisksChanged } from "./lib/tauri";
import { useDisksStore } from "./state/disksStore";
import { DisksPanel } from "./components/DisksPanel";
import { EndpointList } from "./components/EndpointList";
import "./App.css";

function App() {
  const disks = useDisksStore((s) => s.disks);
  const sources = useDisksStore((s) => s.sources);
  const destinations = useDisksStore((s) => s.destinations);
  const setDisks = useDisksStore((s) => s.setDisks);
  const removeSource = useDisksStore((s) => s.removeSource);
  const removeDestination = useDisksStore((s) => s.removeDestination);
  const setSourceLabel = useDisksStore((s) => s.setSourceLabel);
  const setDestinationLabel = useDisksStore((s) => s.setDestinationLabel);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listDisks().then(setDisks).catch(console.error);
    onDisksChanged(setDisks).then((fn) => {
      unlisten = fn;
    });

    return () => unlisten?.();
  }, [setDisks]);

  return (
    <main className="min-h-screen bg-neutral-950 p-6 text-neutral-100">
      <h1 className="mb-6 text-xl font-semibold">OffloadKit</h1>
      <div className="grid grid-cols-1 gap-6 md:grid-cols-3">
        <DisksPanel />
        <EndpointList
          title="Sources"
          endpoints={sources}
          disks={disks}
          onRemove={removeSource}
          onLabelChange={setSourceLabel}
        />
        <EndpointList
          title="Destinations"
          endpoints={destinations}
          disks={disks}
          onRemove={removeDestination}
          onLabelChange={setDestinationLabel}
        />
      </div>
    </main>
  );
}

export default App;
