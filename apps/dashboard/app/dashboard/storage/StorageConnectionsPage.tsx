"use client";

import { Button, Card, Spinner } from "@heroui/react";
import { useState } from "react";
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

  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<
    Record<string, { ok: boolean; message?: string }>
  >({});
  const [error, setError] = useState<string | null>(null);

  const defaultProjectId = projects.data?.[0]?.id;

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
    if (!confirm("Delete this storage connection? Files using it will lose their backend.")) {
      return;
    }
    setError(null);
    try {
      await remove.mutateAsync(id);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Delete failed.");
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <header className="flex items-end justify-between gap-4">
        <div className="flex flex-col gap-1">
          <h1 className="text-3xl font-semibold tracking-tight">
            Storage connections
          </h1>
          <p className="text-default-500">
            Local folders, FTP, and SFTP backends available to your projects.
          </p>
        </div>
        <Button
          variant="primary"
          onPress={() => {
            setEditing(null);
            setCreating((v) => !v);
          }}
          isDisabled={!defaultProjectId}
        >
          {creating ? "Cancel" : "Add connection"}
        </Button>
      </header>

      {error && (
        <div
          role="alert"
          className="rounded-lg border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger"
        >
          {error}
        </div>
      )}

      {creating && defaultProjectId && (
        <Card>
          <Card.Header>
            <Card.Title>New connection</Card.Title>
            <Card.Description>
              Configure a storage backend. Credentials are encrypted at rest.
            </Card.Description>
          </Card.Header>
          <Card.Content>
            <ConnectionForm
              mode="create"
              projectId={defaultProjectId}
              onCancel={() => setCreating(false)}
              onSaved={() => setCreating(false)}
            />
          </Card.Content>
        </Card>
      )}

      {connections.isPending ? (
        <div className="flex justify-center py-12">
          <Spinner />
        </div>
      ) : connections.data && connections.data.length === 0 ? (
        <Card>
          <Card.Content className="py-10 text-center text-sm text-default-500">
            No storage connections yet. Add one to start storing uploaded files.
          </Card.Content>
        </Card>
      ) : (
        <div className="flex flex-col gap-3">
          {connections.data?.map((c) => (
            <ConnectionRow
              key={c.id}
              connection={c}
              isEditing={editing === c.id}
              onEdit={() => {
                setCreating(false);
                setEditing((cur) => (cur === c.id ? null : c.id));
              }}
              onDelete={() => onDelete(c.id)}
              onTest={() => onTest(c.id)}
              testResult={testResult[c.id]}
              testing={test.isPending}
              deleting={remove.isPending}
            />
          ))}
        </div>
      )}
    </div>
  );
}

type RowProps = {
  connection: StorageConnection;
  isEditing: boolean;
  onEdit: () => void;
  onDelete: () => void;
  onTest: () => void;
  testResult?: { ok: boolean; message?: string };
  testing: boolean;
  deleting: boolean;
};

function ConnectionRow({
  connection,
  isEditing,
  onEdit,
  onDelete,
  onTest,
  testResult,
  testing,
  deleting,
}: RowProps) {
  const editingValue: ConnectionFormValue = {
    host: connection.host ?? "",
    port: connection.port ?? undefined,
    username: connection.username ?? "",
    password: "",
    private_key: "",
    base_path: connection.base_path,
    public_base_url: connection.public_base_url,
  };

  return (
    <Card>
      <Card.Content className="flex flex-col gap-4">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="flex flex-col gap-1 min-w-0">
            <div className="flex items-center gap-2">
              <span className="rounded-md bg-default-100 px-2 py-0.5 text-xs font-medium uppercase tracking-wide text-default-600">
                {connection.type}
              </span>
              <span className="text-xs text-default-500">{connection.id}</span>
            </div>
            <h2 className="text-base font-medium truncate">
              {connection.host
                ? `${connection.host}${connection.port ? `:${connection.port}` : ""}`
                : connection.base_path}
            </h2>
            <p className="text-xs text-default-500 truncate">
              {connection.base_path} → {connection.public_base_url}
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="tertiary" onPress={onTest} isPending={testing}>
              Test
            </Button>
            <Button size="sm" variant="tertiary" onPress={onEdit}>
              {isEditing ? "Close" : "Edit"}
            </Button>
            <Button
              size="sm"
              variant="danger-soft"
              onPress={onDelete}
              isPending={deleting}
            >
              Delete
            </Button>
          </div>
        </div>

        {testResult && (
          <div
            role="status"
            className={
              "rounded-lg border px-3 py-2 text-sm " +
              (testResult.ok
                ? "border-success/30 bg-success/10 text-success"
                : "border-danger/30 bg-danger/10 text-danger")
            }
          >
            {testResult.ok
              ? "Connection succeeded."
              : `Connection failed: ${testResult.message ?? "unknown error"}`}
          </div>
        )}

        {isEditing && (
          <div className="rounded-xl border border-default-200 p-4">
            <ConnectionForm
              mode="update"
              connectionId={connection.id}
              connectionType={connection.type}
              value={editingValue}
              onCancel={onEdit}
              onSaved={onEdit}
            />
          </div>
        )}
      </Card.Content>
    </Card>
  );
}
