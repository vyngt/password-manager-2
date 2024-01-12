import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import { WindowTitleBar } from "@/components/Window";

import { ThemeProvider } from "@/components/Theme";

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
        <ThemeProvider>
          <div className="app-none-scrollbar flex h-full flex-col">
            <WindowTitleBar />
            {children}
          </div>
        </ThemeProvider>
      </body>
    </html>
  );
}
