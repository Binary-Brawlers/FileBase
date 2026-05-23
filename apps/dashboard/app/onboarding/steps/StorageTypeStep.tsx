"use client";

import { Button } from "@heroui/react";

export type StorageType = "local" | "ftp" | "sftp";

type Props = {
  value: StorageType;
  onChange: (v: StorageType) => void;
  onBack: () => void;
  onNext: () => void;
};

const OPTIONS: {
  id: StorageType;
  title: string;
  desc: string;
  icon: string;
}[] = [
  {
    id: "local",
    title: "Local folder",
    desc: "Store files on this server. Fastest, no extra setup.",
    icon: "💾",
  },
  {
    id: "ftp",
    title: "FTP server",
    desc: "Upload to an existing FTP host like cPanel or shared hosting.",
    icon: "🌐",
  },
  {
    id: "sftp",
    title: "SFTP server",
    desc: "Upload over SSH to a VPS or managed server.",
    icon: "🔐",
  },
];

export function StorageTypeStep({ value, onChange, onBack, onNext }: Props) {
  return (
    <div className="flex flex-col gap-5">
      <div className="grid gap-3">
        {OPTIONS.map((opt) => {
          const selected = value === opt.id;
          return (
            <button
              key={opt.id}
              type="button"
              onClick={() => onChange(opt.id)}
              className={
                "group relative flex w-full items-start gap-4 rounded-xl border p-4 text-left transition " +
                (selected
                  ? "border-accent bg-accent/5 ring-1 ring-accent"
                  : "border-default-200 hover:border-default-300 hover:bg-default-100/40")
              }
            >
              <span
                className={
                  "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg text-lg " +
                  (selected ? "bg-accent/15" : "bg-default-100")
                }
                aria-hidden
              >
                {opt.icon}
              </span>
              <div className="flex flex-col gap-0.5 min-w-0 flex-1">
                <p className="font-medium">{opt.title}</p>
                <p className="text-xs text-default-500">{opt.desc}</p>
              </div>
              <span
                className={
                  "mt-1 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border-2 " +
                  (selected
                    ? "border-accent bg-accent text-white"
                    : "border-default-300")
                }
                aria-hidden
              >
                {selected && (
                  <span className="h-1.5 w-1.5 rounded-full bg-white" />
                )}
              </span>
            </button>
          );
        })}
      </div>

      <div className="flex justify-end gap-2">
        <Button variant="tertiary" onPress={onBack}>
          Back
        </Button>
        <Button variant="primary" onPress={onNext}>
          Continue
        </Button>
      </div>
    </div>
  );
}
