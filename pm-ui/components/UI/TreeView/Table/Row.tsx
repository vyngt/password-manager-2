"use client";

import "./styles.css";

export function Row({
  children,
  even,
}: {
  children: React.ReactNode;
  even?: boolean;
}) {
  const className = even ? "app--tree--row even" : "app--tree--row";
  return <tr className={className}>{children}</tr>;
}
