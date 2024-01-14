"use client";

import { VaultProvider } from "@/components/_Vault";

export default function MainLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <VaultProvider>{children}</VaultProvider>;
}
