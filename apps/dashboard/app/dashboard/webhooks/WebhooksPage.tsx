"use client";

import { Button, Chip, Input, Label, Switch, TextField, useOverlayState } from "@heroui/react";
import { Copy, Pencil, Plus, Trash2, Webhook } from "lucide-react";
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
import { ApiError, type CreatedWebhook, type Webhook as WebhookRecord, type WebhookEvent } from "../../../lib/api";
import {
  useCreateWebhook,
  useDeleteWebhook,
  useProjects,
  useUpdateWebhook,
  useWebhookDeliveries,
  useWebhooks,
} from "../../../lib/queries";

const EVENTS: WebhookEvent[] = [
  "file.uploaded",
  "file.deleted",
  "file.duplicate_detected",
  "file.optimized",
  "file.failed",
];

export function WebhooksPage() {
  const projects = useProjects();
  const webhooks = useWebhooks();
  const create = useCreateWebhook();
  const update = useUpdateWebhook();
  const remove = useDeleteWebhook();
  const modal = useOverlayState();
  const [editing, setEditing] = useState<WebhookRecord | null>(null);
  const [projectId, setProjectId] = useState("");
  const [url, setUrl] = useState("");
  const [events, setEvents] = useState<WebhookEvent[]>(["file.uploaded"]);
  const [active, setActive] = useState(true);
  const [secret, setSecret] = useState("");
  const [created, setCreated] = useState<CreatedWebhook | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const deliveries = useWebhookDeliveries(selectedId);
  const selectedProjectId = projectId || projects.data?.[0]?.id || "";

  function openCreate() {
    setEditing(null);
    setProjectId(projects.data?.[0]?.id ?? "");
    setUrl("");
    setEvents(["file.uploaded"]);
    setActive(true);
    setSecret("");
    modal.open();
  }

  function openEdit(hook: WebhookRecord) {
    setEditing(hook);
    setProjectId(hook.project_id);
    setUrl(hook.url);
    setEvents(hook.events);
    setActive(hook.is_active);
    setSecret("");
    modal.open();
  }

  function toggleEvent(event: WebhookEvent) {
    setEvents((current) =>
      current.includes(event)
        ? current.filter((item) => item !== event)
        : [...current, event],
    );
  }

  async function save() {
    setError(null);
    try {
      if (editing) {
        await update.mutateAsync({
          id: editing.id,
          body: { url, events, is_active: active, ...(secret ? { secret } : {}) },
        });
      } else {
        const res = await create.mutateAsync({
          project_id: selectedProjectId,
          url,
          events,
          is_active: active,
          ...(secret ? { secret } : {}),
        });
        setCreated(res);
      }
      modal.close();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Webhook save failed.");
    }
  }

  async function onDelete(id: string) {
    if (!confirm("Delete this webhook?")) return;
    setError(null);
    try {
      await remove.mutateAsync(id);
      if (selectedId === id) setSelectedId(null);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Webhook delete failed.");
    }
  }

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        icon={Webhook}
        title="Webhooks"
        description="Notify your application when files are uploaded, deleted, optimized, rejected as duplicates, or fail validation."
        action={
          <Button variant="primary" onPress={openCreate} isDisabled={!selectedProjectId}>
            <Plus className="h-4 w-4" /> New webhook
          </Button>
        }
      />

      {error && <Alert message={error} />}

      {created && (
        <section className="rounded-3xl border border-success/25 bg-success/10 p-5 shadow-sm">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="font-semibold text-success">Copy this signing secret now</h2>
              <p className="text-sm text-default-600">The full secret is only shown after creating the webhook.</p>
            </div>
            <Button size="sm" variant="tertiary" onPress={() => navigator.clipboard?.writeText(created.signing_secret)}>
              <Copy className="h-3.5 w-3.5" /> Copy
            </Button>
          </div>
          <code className="mt-4 block break-all rounded-2xl border border-success/20 bg-background/80 p-4 text-sm">{created.signing_secret}</code>
        </section>
      )}

      {webhooks.isPending ? (
        <LoadingBlock />
      ) : !webhooks.data?.length ? (
        <EmptyBlock icon={Webhook} title="No webhooks" description="Create an endpoint subscription to receive signed file lifecycle events." action={<Button variant="primary" onPress={openCreate}>Create webhook</Button>} />
      ) : (
        <div className="grid gap-5 xl:grid-cols-[minmax(0,1.1fr)_minmax(360px,0.9fr)]">
          <div className="grid gap-4">
            {webhooks.data.map((hook) => (
              <article key={hook.id} className="rounded-3xl border border-default-200 bg-background p-5 shadow-sm">
                <div className="flex flex-wrap items-start justify-between gap-4">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <h2 className="break-all font-semibold">{hook.url}</h2>
                      <Chip size="sm" variant="soft" color={hook.is_active ? "success" : "default"}>{hook.is_active ? "Active" : "Paused"}</Chip>
                    </div>
                    <p className="mt-1 font-mono text-xs text-default-400">{hook.id}</p>
                  </div>
                  <div className="flex gap-2">
                    <Button size="sm" variant="tertiary" onPress={() => setSelectedId(hook.id)}>Deliveries</Button>
                    <Button size="sm" variant="tertiary" onPress={() => openEdit(hook)}><Pencil className="h-3.5 w-3.5" /> Edit</Button>
                    <Button size="sm" variant="danger-soft" onPress={() => onDelete(hook.id)} isPending={remove.isPending}><Trash2 className="h-3.5 w-3.5" /></Button>
                  </div>
                </div>
                <div className="mt-4 flex flex-wrap gap-2">
                  {hook.events.map((event) => <Chip key={event} size="sm" variant="soft">{event}</Chip>)}
                </div>
                <p className="mt-4 text-xs text-default-500">Updated {new Date(hook.updated_at).toLocaleString()}</p>
              </article>
            ))}
          </div>

          <aside className="rounded-3xl border border-default-200 bg-background p-5 shadow-sm">
            <h2 className="font-semibold">Delivery logs</h2>
            <p className="mt-1 text-sm text-default-500">Select a webhook to inspect recent delivery attempts.</p>
            {!selectedId ? (
              <p className="mt-6 rounded-2xl bg-default-50 p-4 text-sm text-default-500">No webhook selected.</p>
            ) : deliveries.isPending ? (
              <LoadingBlock />
            ) : !deliveries.data?.length ? (
              <p className="mt-6 rounded-2xl bg-default-50 p-4 text-sm text-default-500">No deliveries recorded yet.</p>
            ) : (
              <div className="mt-5 flex max-h-[34rem] flex-col gap-3 overflow-auto pr-1">
                {deliveries.data.map((log) => (
                  <div key={log.id} className="rounded-2xl border border-default-100 bg-default-50 p-4">
                    <div className="flex items-center justify-between gap-2">
                      <Chip size="sm" variant="soft" color={log.status === "delivered" ? "success" : "danger"}>{log.status}</Chip>
                      <span className="text-xs text-default-500">Attempt {log.attempt}</span>
                    </div>
                    <p className="mt-2 text-sm font-medium">{log.event}</p>
                    <p className="mt-1 text-xs text-default-500">{new Date(log.created_at).toLocaleString()} {log.status_code ? `- HTTP ${log.status_code}` : ""}</p>
                    {log.error && <p className="mt-2 text-xs text-danger">{log.error}</p>}
                  </div>
                ))}
              </div>
            )}
          </aside>
        </div>
      )}

      <FormModal state={modal} title={editing ? "Edit webhook" : "Create webhook"} description="Choose the endpoint and events FileBase should deliver." size="lg">
        <div className="flex flex-col gap-4">
          {!editing && (
            <label className="flex flex-col gap-1.5 text-sm">
              <span className="font-medium text-default-700">Project</span>
              <NativeSelect value={selectedProjectId} onChange={(e) => setProjectId(e.target.value)}>
                {projects.data?.map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}
              </NativeSelect>
            </label>
          )}
          <TextField isRequired>
            <Label>Endpoint URL</Label>
            <Input value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://example.com/api/filebase-webhook" />
          </TextField>
          <TextField>
            <Label>{editing ? "Rotate signing secret" : "Signing secret"}</Label>
            <Input value={secret} onChange={(e) => setSecret(e.target.value)} placeholder={editing ? "Leave blank to keep current secret" : "Auto-generated if blank"} />
          </TextField>
          <div className="flex items-center justify-between rounded-2xl border border-default-100 p-3">
            <div>
              <p className="text-sm font-medium">Active</p>
              <p className="text-xs text-default-500">Paused webhooks are not queued.</p>
            </div>
            <Switch isSelected={active} onChange={(on) => setActive(on)}>
              <Switch.Control>
                <Switch.Thumb />
              </Switch.Control>
            </Switch>
          </div>
          <div className="flex flex-col gap-2">
            <span className="text-sm font-medium text-default-700">Events</span>
            <div className="grid gap-2 sm:grid-cols-2">
              {EVENTS.map((event) => (
                <label key={event} className="flex items-center gap-2 rounded-2xl border border-default-100 p-3 text-sm">
                  <input type="checkbox" checked={events.includes(event)} onChange={() => toggleEvent(event)} />
                  <span>{event}</span>
                </label>
              ))}
            </div>
          </div>
          <ModalActions>
            <Button variant="tertiary" onPress={modal.close}>Cancel</Button>
            <Button variant="primary" onPress={save} isPending={create.isPending || update.isPending} isDisabled={!url.trim() || !events.length || !selectedProjectId}>Save webhook</Button>
          </ModalActions>
        </div>
      </FormModal>
    </div>
  );
}
