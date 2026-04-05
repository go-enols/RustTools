import { ReactNode } from 'react';

interface AppShellProps {
  sidebar: ReactNode;
  children: ReactNode;
}

export default function AppShell({ sidebar, children }: AppShellProps) {
  return (
    <div className="app-shell">
      {/* Left: Sidebar */}
      <div className="app-left">
        {sidebar}
      </div>

      {/* Center: Main Content */}
      <div className="app-main">
        <main className="main-content">
          {children}
        </main>
      </div>
    </div>
  );
}
