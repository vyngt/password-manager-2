"use client";

import { useCurrentTheme } from "@/components/Theme";
import { Sidebar } from "@/components/Sidebar";

export const metadata = {
  title: "Main",
  description: "Main App Window",
};

export default function MainLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  useCurrentTheme();

  return (
    <div className="flex h-full w-full">
      <Sidebar className="h-full w-[20rem]" />
      <main className="h-full w-full">{children}</main>
    </div>
  );
}
