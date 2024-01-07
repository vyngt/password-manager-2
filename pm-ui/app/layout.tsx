import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import { WindowTitleBar } from "@/components/Window";
import { Sidebar } from "@/components/Panel";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Password Manager",
  description: "Password Manager",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <div className="flex h-full flex-col">
          <WindowTitleBar />
          <div className="root-layout relative flex w-full">
            <Sidebar className="app-none-scrollbar h-full w-32" />
            <div className="app-scrollbar flex-1 overflow-y-auto">
              {children}
            </div>
          </div>
        </div>
      </body>
    </html>
  );
}
