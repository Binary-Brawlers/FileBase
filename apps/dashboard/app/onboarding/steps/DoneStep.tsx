"use client";

import { Button } from "@heroui/react";
import { API_BASE_URL } from "../../../lib/config";

type Props = { onContinue: () => void };

export function DoneStep({ onContinue }: Props) {
  const dashboardUrl =
    typeof window !== "undefined" ? window.location.origin : "";
  const snippet = `import { FileBaseClient } from "@filebase/client";

const client = new FileBaseClient({
  signEndpoint: "/api/upload/sign",
});

await client.upload(file, { preset: "default" });`;

  return (
    <div className="flex flex-col gap-5">
      <div className="flex items-center gap-3 rounded-xl border border-success/30 bg-success/10 px-4 py-3">
        <span
          className="flex h-9 w-9 items-center justify-center rounded-full bg-success/20 text-success"
          aria-hidden
        >
          ✓
        </span>
        <div>
          <p className="font-medium">FileBase is ready</p>
          <p className="text-xs text-default-500">
            Your install is initialised and you&apos;re signed in.
          </p>
        </div>
      </div>

      <div className="grid gap-3">
        <Field label="API URL" value={API_BASE_URL} mono />
        <Field label="Dashboard URL" value={dashboardUrl || "—"} mono />
        <div>
          <p className="text-xs uppercase tracking-wide text-default-500 mb-1.5">
            SDK snippet
          </p>
          <pre className="block whitespace-pre overflow-auto rounded-lg border border-default-200 bg-default-100/60 p-3 text-xs font-mono leading-relaxed">
            {snippet}
          </pre>
        </div>
      </div>

      <div className="flex justify-end">
        <Button variant="primary" onPress={onContinue}>
          Open dashboard
        </Button>
      </div>
    </div>
  );
}

function Field({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div>
      <p className="text-xs uppercase tracking-wide text-default-500 mb-1.5">
        {label}
      </p>
      <div
        className={
          "w-full overflow-auto rounded-lg border border-default-200 bg-default-100/40 px-3 py-2 text-sm " +
          (mono ? "font-mono" : "")
        }
      >
        {value}
      </div>
    </div>
  );
}
