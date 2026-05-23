"use client";

import { Button, Chip, useOverlayState } from "@heroui/react";
import { CheckCircle2, Database, Folder, Globe2, Pencil, Plus, Server, Trash2, Wifi, type LucideIcon } from "lucide-react";
import { useState } from "react";
import {
  Alert,
  EmptyBlock,
  FormModal,
  LoadingBlock,
  PageHeader,
} from "../../../components/PageUI";
import { ApiError, type StorageConnection } from "../../../lib/api";
import {
  useDeleteStorageConnection,
  useProjects,
  useStorageConnections,
  useTestStorageConnection,
} from "../../../lib/queries";
import { ConnectionForm, type ConnectionFormValue } from "./ConnectionForm";

export function StorageConnectionsPage() {
  const projects = useProjects();
  const connections = useStorageConnections();
  const remove = useDeleteStorageConnection();
  const test = useTestStorageConnection();
  const createModal = useOverlayState();
  const editModal = useOverlayState();
  const [editing, setEditing] = useState<StorageConnection | null>(null);
  const [testResult, setTestResult] = useState<Record<string, { ok: boolean; message?: string }>>({});
  const [error, setError] = useState<string | null>(null);

  const defaultProjectId = projects.data?.[0]?.id;

  function openEdit(connection: StorageConnection) {
    setEditing(connection);
    editModal.open();
  }

  async function onTest(id: string) {
    setError(null);
    try {
      const res = await test.mutateAsync(id);
      setTestResult((r) => ({ ...r, [id]: res }));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Test failed.");
    }
  }

  async function onDelete(id: string) {
    if (!confirm("Delete this storage connection? Files using it will lose their backend.")) return;
    setError(null);
    try {
      await remove.mutateAsync(id);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Delete failed.");
    }
  }

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        icon={Database}
        title="Storage connections"
        description="Connect local folders, FTP, and SFTP destinations. Credentials stay encrypted and are never exposed to client apps."
        action={
          <Button variant="primary" onPress={createModal.open} isDisabled={!defaultProjectId}>
            <Plus className="h-4 w-4" /> Add connection
          </Button>
        }
      />

      {error && <Alert message={error} />}

      {connections.isPending ? (
        <LoadingBlock />
      ) : !connections.data?.length ? (
        <EmptyBlock
          icon={Database}
          title="No storage connections"
          description="Add a local, FTP, or SFTP destination before creating production upload presets."
          action={
            <Button variant="primary" onPress={createModal.open} isDisabled={!defaultProjectId}>
              <Plus className="h-4 w-4" /> Add connection
            </Button>
          }
        />
      ) : (
        <div className="grid gap-4 xl:grid-cols-2">
          {connections.data.map((connection) => (
            <ConnectionCard
              key={connection.id}
              connection={connection}
              testResult={testResult[connection.id]}
              testing={test.isPending}
              deleting={remove.isPending}
              onEdit={() => openEdit(connection)}
              onDelete={() => onDelete(connection.id)}
              onTest={() => onTest(connection.id)}
            />
          ))}
        </div>
      )}

      <FormModal
        state={createModal}
        title="Add storage connection"
        description="Choose a storage driver and configure its destination."
        size="lg"
      >
        {defaultProjectId && (
          <ConnectionForm
            mode="create"
            projectId={defaultProjectId}
            onCancel={createModal.close}
            onSaved={createModal.close}
          />
        )}
      </FormModal>

      <FormModal
        state={editModal}
        title="Edit storage connection"
        description="Update the destination, credentials, or public URL."
        size="lg"
      >
        {editing && (
          <ConnectionForm
            mode="update"
            connectionId={editing.id}
            connectionType={editing.type}
            value={toFormValue(editing)}
            onCancel={editModal.close}
            onSaved={editModal.close}
          />
        )}
      </FormModal>
    </div>
  );
}

function ConnectionCard({
  connection,
  testResult,
  testing,
  deleting,
  onEdit,
  onDelete,
  onTest,
}: {
  connection: StorageConnection;
  testResult?: { ok: boolean; message?: string };
  testing: boolean;
  deleting: boolean;
  onEdit: () => void;
  onDelete: () => void;
  onTest: () => void;
}) {
  const title = connection.host
    ? `${connection.host}${connection.port ? `:${connection.port}` : ""}`
    : connection.base_path;

  return (
    <article className="group relative overflow-hidden rounded-3xl border border-default-200 bg-background p-5 shadow-sm transition hover:-translate-y-0.5 hover:border-accent/40 hover:shadow-md">
      <div className="absolute inset-x-0 top-0 h-1 bg-gradient-to-r from-accent to-primary opacity-80" />
      <div className="flex flex-col gap-5">
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-start gap-3 min-w-0">
            <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-default-100 text-default-600">
              {connection.type === "local" ? <Folder className="h-5 w-5" /> : <Server className="h-5 w-5" />}
            </div>
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="truncate text-base font-semibold">{title}</h2>
                <Chip size="sm" variant="soft" color="default">{connection.type.toUpperCase()}</Chip>
              </div>
              <p className="mt-1 truncate font-mono text-xs text-default-400">{connection.id}</p>
            </div>
          </div>
          <Button size="sm" variant="tertiary" onPress={onTest} isPending={testing}>
            <Wifi className="h-3.5 w-3.5" /> Test
          </Button>
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <InfoTile icon={Folder} label="Base path" value={connection.base_path} />
          <InfoTile icon={Globe2} label="Public URL" value={connection.public_base_url} />
        </div>

        {testResult && (
          <div className={"rounded-2xl border px-3 py-2 text-sm " + (testResult.ok ? "border-success/25 bg-success/10 text-success" : "border-danger/25 bg-danger/10 text-danger")}>
            <div className="flex items-center gap-2 font-medium">
              <CheckCircle2 className="h-4 w-4" />
              {testResult.ok ? "Connection succeeded" : "Connection failed"}
            </div>
            {!testResult.ok && <p className="mt-1 text-xs">{testResult.message ?? "Unknown error"}</p>}
          </div>
        )}

        <div className="flex justify-end gap-2 border-t border-default-100 pt-4">
          <Button size="sm" variant="tertiary" onPress={onEdit}>
            <Pencil className="h-3.5 w-3.5" /> Edit
          </Button>
          <Button size="sm" variant="danger-soft" onPress={onDelete} isPending={deleting}>
            <Trash2 className="h-3.5 w-3.5" /> Delete
          </Button>
        </div>
      </div>
    </article>
  );
}

function InfoTile({ icon: Icon, label, value }: { icon: LucideIcon; label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-default-100 bg-default-50 p-3">
      <div className="mb-1 flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-default-500">
        <Icon className="h-3.5 w-3.5" /> {label}
      </div>
      <p className="truncate text-sm text-default-700">{value}</p>
    </div>
  );
}

function toFormValue(connection: StorageConnection): ConnectionFormValue {
  return {
    host: connection.host ?? "",
    port: connection.port ?? undefined,
    username: connection.username ?? "",
    password: "",
    private_key: "",
    base_path: connection.base_path,
    public_base_url: connection.public_base_url,
  };
}
