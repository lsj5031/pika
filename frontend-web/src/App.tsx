import { SessionList, SessionHistory } from "./components";
import { useAppStore } from "./store/appStore";

function App() {
  const currentSessionId = useAppStore((state) => state.currentSessionId);

  return (
    <div className="flex h-screen w-full">
      {/* Sidebar - SessionList */}
      <aside className="w-64 border-r bg-background">
        <SessionList />
      </aside>

      {/* Main content area */}
      <main className="flex-1">
        <SessionHistory sessionId={currentSessionId} />
      </main>
    </div>
  );
}

export default App;
