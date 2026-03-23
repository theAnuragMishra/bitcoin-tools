import { useState } from "react";

export default function Section({
  title,
  children,
  defaultOpen = true,
}: {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="section">
      <div className="section-hdr" onClick={() => setOpen(!open)}>
        <h3>{title}</h3>
        <span className="toggle">{open ? "▲" : "▼"}</span>
      </div>
      {open && <div className="section-body">{children}</div>}
    </div>
  );
}
