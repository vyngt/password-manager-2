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
    <div className="relative h-full w-full">
      <Sidebar className="border-pm-foreground fixed h-full w-[16rem] border-r border-solid" />
      <main className="ml-[16rem] mr-[2px]">{children}</main>
    </div>
  );
}
