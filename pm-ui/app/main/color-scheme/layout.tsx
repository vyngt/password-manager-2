"use client";

import { Provider } from "@/components/ColorScheme";

export default function ColorSchemeLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <Provider>{children}</Provider>;
}
