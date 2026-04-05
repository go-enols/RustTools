import { useState } from 'react';

export default function Footer() {
  const [status] = useState('就绪');
  const [gpu] = useState('GPU: RTX 3080');
  const [memory] = useState('内存: 4.2 GB / 16 GB');

  return (
    <footer className="status-bar">
      <div className="flex items-center gap-lg">
        <span className="flex items-center gap-sm">
          <span className="status-dot"></span>
          {status}
        </span>
      </div>
      <div className="flex items-center gap-lg">
        <span>{gpu}</span>
        <span>{memory}</span>
      </div>
    </footer>
  );
}
