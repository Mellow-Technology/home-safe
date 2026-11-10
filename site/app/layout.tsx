import type { Metadata } from "next";
import { Fraunces, IBM_Plex_Mono, Nunito } from "next/font/google";
import { ThemeScript } from "gui-mellow/ThemeToggle";
import "./theme.scss";
import "./site.scss";

// Fraunces' SOFT axis rounds its serifs, for headings that feel hand-lettered.
const display = Fraunces({ variable: "--font-display", subsets: ["latin"], axes: ["SOFT", "opsz"] });
const body = Nunito({ variable: "--font-body", subsets: ["latin"] });
const mono = IBM_Plex_Mono({ variable: "--font-mono", subsets: ["latin"], weight: ["400"] });

export const metadata: Metadata = {
  title: "Home Safe · Keep every home you fall for",
  description:
    "Save Zillow listings in one click, with every photo, to a private home inventory on your own computer. Free and open source.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${display.variable} ${body.variable} ${mono.variable}`} suppressHydrationWarning>
      <head>
        <ThemeScript />
      </head>
      <body>{children}</body>
    </html>
  );
}
