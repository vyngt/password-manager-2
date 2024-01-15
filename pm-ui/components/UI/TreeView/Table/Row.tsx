"use client";

import "./styles.css";

export function Row({ children }: { children: React.ReactNode }) {
  return <tr className="app--tree--row">{children}</tr>;
}
