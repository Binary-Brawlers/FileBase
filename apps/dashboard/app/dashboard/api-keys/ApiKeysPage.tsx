"use client";

import { Button, Chip, Input, Label, TextField, useOverlayState } from "@heroui/react";
import { Copy, EyeOff, KeyRound, LockKeyhole, Plus, ShieldCheck, Trash2 } from "lucide-react";
import { useState } from "react";
import {
  Alert,
  EmptyBlock,
  FormModal,
  LoadingBlock,
  ModalActions,
  NativeSelect,
  PageHeader,
} from "../../../components/PageUI";
import { ApiError, type CreatedApiKey } from "../../../lib/api";
import { useApiKeys, useCreateApiKey, useProjects, useRevokeApiKey } from "../../../lib/queries";

export function ApiKeysPage() {
  const projects = useProjects();
  const keys = useApiKeys();
  const create = useCreateApiKey();
  const revoke = useRevokeApiKey();
  const createModal = useOverlayState();
  const [name, setName] = useState("Default server key");
  const [mode, setMode] = useState<"live" | "test">("live");
  const [projectId, setProjectId] = useState("");
  const [created, setCreated] = useState<CreatedApiKey | null>(null);
  const [error, setError] = useState<string | null>(null);

  const selectedProjectId = projectId || projects.data?.[0]?.id || "";

  async function onCreate() {
    setError(null);
    setCreated(null);
    try {
      const res = await create.mutateAsync({ project_id: selectedProjectId, name, mode });
      setCreated(res);
      createModal.close();
      setName("Default server key");
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Create failed.");
    }
  }

  async function onRevoke(id: string) {
    if (!confirm("Revoke this API key? Applications using it will stop working.")) return;
    setError(null);
    try {
      await revoke.mutateAsync(id);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Revoke failed.");
    }
  }

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        icon={KeyRound}
        title="API keys"
        description="Generate private server keys for upload signing and backend integrations. Full secrets are shown once, then only hashed."
        action={
          <Button variant="primary" onPress={createModal.open} isDisabled={!selectedProjectId}>
            <Plus className="h-4 w-4" /> New key
          </Button>
        }
      />

      {error && <Alert message={error} />}

      {created && (
        <section className="relative overflow-hidden rounded-3xl border border-success/25 bg-success/10 p-5 shadow-sm">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
            <div className="flex items-start gap-3">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-success/15 text-success">
                <ShieldCheck className="h-5 w-5" />
              </div>
              <div>
                <h2 className="font-semibold text-success">Copy this API key now</h2>
                <p className="text-sm text-default-600">The full secret is only returned once. Store it in your application environment variables.</p>
              </div>
            </div>
            <Button size="sm" variant="tertiary" onPress={() => navigator.clipboard?.writeText(created.secret)}>
              <Copy className="h-3.5 w-3.5" /> Copy
            </Button>
          </div>
          <code className="mt-4 block break-all rounded-2xl border border-success/20 bg-background/80 p-4 text-sm text-default-800">{created.secret}</code>
        </section>
      )}

      {keys.isPending ? (
        <LoadingBlock />
      ) : !keys.data?.length ? (
        <EmptyBlock
          icon={KeyRound}
          title="No API keys"
          description="Create a test or live key to start integrating FileBase from your backend."
          action={
            <Button variant="primary" onPress={createModal.open} isDisabled={!selectedProjectId}>
              <Plus className="h-4 w-4" /> New key
            </Button>
          }
        />
      ) : (
        <div className="grid gap-4 xl:grid-cols-2">
          {keys.data.map((key) => {
            const isLive = key.prefix.startsWith("fb_live_");
            return (
              <article key={key.id} className="rounded-3xl border border-default-200 bg-background p-5 shadow-sm transition hover:-translate-y-0.5 hover:border-accent/40 hover:shadow-md">
                <div className="flex items-start justify-between gap-4">
                  <div className="flex items-start gap-3 min-w-0">
                    <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-default-100 text-default-600">
                      <LockKeyhole className="h-5 w-5" />
                    </div>
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <h2 className="truncate text-base font-semibold">{key.name}</h2>
                        <Chip size="sm" variant="soft" color={isLive ? "success" : "default"}>{isLive ? "Live" : "Test"}</Chip>
                        {key.revoked_at && <Chip size="sm" variant="soft" color="danger">Revoked</Chip>}
                      </div>
                      <p className="mt-1 font-mono text-xs text-default-400">{key.id}</p>
                    </div>
                  </div>
                  <Button size="sm" variant="danger-soft" onPress={() => onRevoke(key.id)} isPending={revoke.isPending} isDisabled={!!key.revoked_at}>
                    <Trash2 className="h-3.5 w-3.5" /> Revoke
                  </Button>
                </div>

                <div className="mt-5 rounded-2xl border border-default-100 bg-default-50 p-4">
                  <div className="mb-2 flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-default-500">
                    <EyeOff className="h-3.5 w-3.5" /> Stored prefix
                  </div>
                  <code className="text-sm text-default-700">{key.prefix}...</code>
                </div>

                <div className="mt-4 flex flex-wrap gap-4 text-xs text-default-500">
                  <span>Created {new Date(key.created_at).toLocaleString()}</span>
                  <span>Last used {key.last_used_at ? new Date(key.last_used_at).toLocaleString() : "never"}</span>
                </div>
              </article>
            );
          })}
        </div>
      )}

      <FormModal
        state={createModal}
        title="Create API key"
        description="Choose a project and environment. The secret will be shown once after creation."
      >
        <div className="flex flex-col gap-4">
          <TextField isRequired>
            <Label>Name</Label>
            <Input value={name} onChange={(e) => setName(e.target.value)} />
          </TextField>
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="flex flex-col gap-1.5 text-sm">
              <span className="font-medium text-default-700">Project</span>
              <NativeSelect value={selectedProjectId} onChange={(e) => setProjectId(e.target.value)}>
                {projects.data?.map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}
              </NativeSelect>
            </label>
            <label className="flex flex-col gap-1.5 text-sm">
              <span className="font-medium text-default-700">Mode</span>
              <NativeSelect value={mode} onChange={(e) => setMode(e.target.value as "live" | "test")}>
                <option value="live">Live</option>
                <option value="test">Test</option>
              </NativeSelect>
            </label>
          </div>
          <ModalActions>
            <Button variant="tertiary" onPress={createModal.close}>Cancel</Button>
            <Button variant="primary" onPress={onCreate} isPending={create.isPending} isDisabled={!name.trim() || !selectedProjectId}>Create key</Button>
          </ModalActions>
        </div>
      </FormModal>
    </div>
  );
}
