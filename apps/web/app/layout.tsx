import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import { ThemeProvider } from "./components/ThemeProvider";
import { HouseholdProvider } from "./components/HouseholdProvider";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "TwoCents - Finance App for Couples",
  description: "Track expenses, manage splits, and achieve savings goals together",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className={inter.className}>
        <ThemeProvider>
          <HouseholdProvider>
            {children}
          </HouseholdProvider>
        </ThemeProvider>
        {/* Required by @glideapps/glide-data-grid overlay editor (cell editing) */}
        <div id="portal" />
      </body>
    </html>
  );
}

