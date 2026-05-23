"use client";

import { Card } from "@heroui/react";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { ApiError, type InitializeRequest } from "../../lib/api";
import { setToken } from "../../lib/auth";
import { useInitialize, useLogin } from "../../lib/queries";
import { AdminStep, type AdminData } from "./steps/AdminStep";
import { StorageTypeStep, type StorageType } from "./steps/StorageTypeStep";
import { StorageConfigStep, type StorageData } from "./steps/StorageConfigStep";
import { PresetStep, type PresetData } from "./steps/PresetStep";
import { DoneStep } from "./steps/DoneStep";

const STEPS = [
  { label: "Admin", hint: "Create your account" },
  { label: "Storage type", hint: "Pick a backend" },
  { label: "Storage", hint: "Connection details" },
  { label: "Preset", hint: "Default upload rule" },
  { label: "Done", hint: "Start using FileBase" },
] as const;

export function Onboarding() {
  const router = useRouter();
  const initialize = useInitialize();
  const login = useLogin();
  const [step, setStep] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const [admin, setAdmin] = useState<AdminData>({
    name: "",
    email: "",
    password: "",
    projectName: "Default Project",
  });
  const [storageType, setStorageType] = useState<StorageType>("local");
  const [storage, setStorage] = useState<StorageData>({
    base_path: "/var/filebase/uploads",
    public_base_url: "http://localhost:8080/uploads",
  });
  const [preset, setPreset] = useState<PresetData>({
    name: "default",
    folder: "uploads",
    max_file_size: 10_485_760,
    duplicate_strategy: "return_existing",
  });

  async function submit() {
    setError(null);
    try {
      const payload: InitializeRequest = {
        admin: {
          name: admin.name,
          email: admin.email,
          password: admin.password,
        },
        project: { name: admin.projectName },
        storage: buildStorage(storageType, storage),
        preset: {
          name: preset.name,
          folder: preset.folder,
          max_file_size: preset.max_file_size,
          duplicate_strategy: preset.duplicate_strategy,
        },
      };
      await initialize.mutateAsync(payload);
      const session = await login.mutateAsync({
        email: admin.email,
        password: admin.password,
      });
      setToken(session.token);
      setStep(4);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Setup failed.");
    }
  }

  const submitting = initialize.isPending || login.isPending;
  const current = STEPS[step];

  return (
    <div className="w-full max-w-2xl flex flex-col gap-6">
      <Stepper step={step} />

      <Card className="shadow-xl border border-default-200/60">
        <Card.Header className="flex flex-col gap-1">
          <Card.Title className="text-2xl">{current.label}</Card.Title>
          <Card.Description>{current.hint}</Card.Description>
        </Card.Header>
        <Card.Content className="gap-5">
          {error && (
            <div
              role="alert"
              className="rounded-lg border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger"
            >
              {error}
            </div>
          )}

          {step === 0 && (
            <AdminStep value={admin} onChange={setAdmin} onNext={() => setStep(1)} />
          )}
          {step === 1 && (
            <StorageTypeStep
              value={storageType}
              onChange={(v) => {
                setStorageType(v);
                setStorage(defaultStorageFor(v));
              }}
              onBack={() => setStep(0)}
              onNext={() => setStep(2)}
            />
          )}
          {step === 2 && (
            <StorageConfigStep
              type={storageType}
              value={storage}
              onChange={setStorage}
              onBack={() => setStep(1)}
              onNext={() => setStep(3)}
            />
          )}
          {step === 3 && (
            <PresetStep
              value={preset}
              onChange={setPreset}
              submitting={submitting}
              onBack={() => setStep(2)}
              onSubmit={submit}
            />
          )}
          {step === 4 && (
            <DoneStep onContinue={() => router.push("/dashboard")} />
          )}
        </Card.Content>
      </Card>
    </div>
  );
}

function Stepper({ step }: { step: number }) {
  return (
    <ol className="flex items-center gap-2">
      {STEPS.map((s, i) => {
        const state =
          i < step ? "done" : i === step ? "active" : "upcoming";
        return (
          <li key={s.label} className="flex-1 flex items-center gap-2">
            <div className="flex items-center gap-2 min-w-0">
              <span
                className={
                  "flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-xs font-semibold transition " +
                  (state === "done"
                    ? "bg-accent text-white"
                    : state === "active"
                      ? "bg-accent/15 text-accent ring-2 ring-accent"
                      : "bg-default-200 text-default-500")
                }
              >
                {state === "done" ? "✓" : i + 1}
              </span>
              <span
                className={
                  "hidden sm:block text-xs font-medium truncate " +
                  (state === "upcoming" ? "text-default-500" : "text-foreground")
                }
              >
                {s.label}
              </span>
            </div>
            {i < STEPS.length - 1 && (
              <div
                className={
                  "h-px flex-1 " +
                  (i < step ? "bg-accent" : "bg-default-200")
                }
              />
            )}
          </li>
        );
      })}
    </ol>
  );
}

function defaultStorageFor(type: StorageType): StorageData {
  if (type === "local") {
    return {
      base_path: "/var/filebase/uploads",
      public_base_url: "http://localhost:8080/uploads",
    };
  }
  if (type === "ftp") {
    return {
      host: "",
      port: 21,
      username: "",
      password: "",
      base_path: "/public_html/uploads",
      public_base_url: "",
    };
  }
  return {
    host: "",
    port: 22,
    username: "",
    password: "",
    private_key: "",
    base_path: "/var/www/uploads",
    public_base_url: "",
  };
}

function buildStorage(
  type: StorageType,
  data: StorageData,
): InitializeRequest["storage"] {
  if (type === "local") {
    return {
      type: "local",
      base_path: data.base_path!,
      public_base_url: data.public_base_url!,
    };
  }
  if (type === "ftp") {
    return {
      type: "ftp",
      host: data.host!,
      port: data.port,
      username: data.username!,
      password: data.password!,
      base_path: data.base_path!,
      public_base_url: data.public_base_url!,
    };
  }
  return {
    type: "sftp",
    host: data.host!,
    port: data.port,
    username: data.username!,
    password: data.password || undefined,
    private_key: data.private_key || undefined,
    base_path: data.base_path!,
    public_base_url: data.public_base_url!,
  };
}
