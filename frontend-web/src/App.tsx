import { SessionList } from "./components";

function App() {
  return (
    <div className="flex h-screen w-full">
      {/* Sidebar - SessionList */}
      <aside className="w-64 border-r bg-background">
        <SessionList />
      </aside>

      {/* Main content area */}
      <main className="flex-1 flex items-center justify-center">
        <p className="text-muted-foreground">Select a session to view history</p>
      </main>
    </div>
  );
}

export default App;
