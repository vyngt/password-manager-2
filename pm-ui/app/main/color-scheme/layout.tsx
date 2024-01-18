"use client";

import { Provider } from "@/components/Features/ColorScheme";

export default function ColorSchemeLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <Provider>{children}</Provider>;
}
