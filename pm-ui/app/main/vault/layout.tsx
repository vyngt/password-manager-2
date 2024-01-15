"use client";

import { Provider } from "@/components/Vault";

export default function VaultLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <Provider>{children}</Provider>;
}
