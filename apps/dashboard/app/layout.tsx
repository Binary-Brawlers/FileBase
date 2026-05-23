import type { Metadata } from "next";
import "./styles.css";

export const metadata: Metadata = {
  title: "FileBase Dashboard",
  description: "Manage FileBase uploads, presets, and storage connections.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
