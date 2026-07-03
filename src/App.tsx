import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4 bg-neutral-950 text-neutral-100">
      <h1 className="text-2xl font-semibold">OffloadKit — scaffold OK</h1>
      <p className="text-sm text-neutral-400">
        Tauri + React + TypeScript + Tailwind CSS
      </p>

      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          className="rounded border border-neutral-700 bg-neutral-900 px-3 py-1"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
        />
        <button
          type="submit"
          className="rounded bg-neutral-100 px-3 py-1 text-neutral-900"
        >
          Greet (IPC smoke test)
        </button>
      </form>
      <p>{greetMsg}</p>
    </main>
  );
}

export default App;
