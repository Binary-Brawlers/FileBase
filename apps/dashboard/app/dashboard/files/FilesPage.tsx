"use client";

import { Button, Chip, Input } from "@heroui/react";
import {
  CalendarDays,
  Copy,
  ExternalLink,
  FileSearch,
  FolderOpen,
  Info,
  Search,
  Trash2,
} from "lucide-react";
import { useDeferredValue, useState } from "react";
import {
  Alert,
  EmptyBlock,
  FieldShell,
  LoadingBlock,
  NativeSelect,
  PageHeader,
} from "../../../components/PageUI";
import { ApiError, type FileFilters, type FileRecord } from "../../../lib/api";
import {
  useDeleteFile,
  useFileLogs,
  useFiles,
  useProjects,
} from "../../../lib/queries";

export function FilesPage() {
  const projects = useProjects();
  const remove = useDeleteFile();
  const [projectId, setProjectId] = useState("");
  const [search, setSearch] = useState("");
  const [mimeType, setMimeType] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [selected, setSelected] = useState<FileRecord | null>(null);
  const [error, setError] = useState<string | null>(null);
  const deferredSearch = useDeferredValue(search);
  const filters: FileFilters = {
    project_id: projectId || undefined,
    search: deferredSearch || undefined,
    mime_type: mimeType || undefined,
    from: from ? new Date(`${from}T00:00:00`).toISOString() : undefined,
    to: to ? new Date(`${to}T23:59:59`).toISOString() : undefined,
  };
  const files = useFiles(filters);
  const logs = useFileLogs(selected?.id);
  const mimeTypes = Array.from(new Set(files.data?.map((file) => file.mime_type) ?? [])).sort();

  async function onDelete(file: FileRecord) {
    if (!confirm(`Delete ${file.original_name} from storage and FileBase?`)) return;
    setError(null);
    try {
      await remove.mutateAsync(file.id);
      if (selected?.id === file.id) setSelected(null);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Delete failed.");
    }
  }

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        icon={FolderOpen}
        title="Files"
        description="Browse uploaded files, copy public URLs, inspect metadata, and remove files from storage."
      />

      {error && <Alert message={error} />}

      <section className="grid gap-3 rounded-3xl border border-default-200 bg-background p-4 shadow-sm lg:grid-cols-[1.3fr_1fr_1fr_1fr_1fr]">
        <FieldShell label="Search">
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-default-400" />
            <Input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
              placeholder="Filename or path"
            />
          </div>
        </FieldShell>
        <FieldShell label="Project">
          <NativeSelect value={projectId} onChange={(e) => setProjectId(e.target.value)}>
            <option value="">All projects</option>
            {projects.data?.map((project) => (
              <option key={project.id} value={project.id}>{project.name}</option>
            ))}
          </NativeSelect>
        </FieldShell>
        <FieldShell label="MIME type">
          <NativeSelect value={mimeType} onChange={(e) => setMimeType(e.target.value)}>
            <option value="">All types</option>
            {mimeTypes.map((type) => (
              <option key={type} value={type}>{type}</option>
            ))}
          </NativeSelect>
        </FieldShell>
        <FieldShell label="From">
          <Input type="date" value={from} onChange={(e) => setFrom(e.target.value)} />
        </FieldShell>
        <FieldShell label="To">
          <Input type="date" value={to} onChange={(e) => setTo(e.target.value)} />
        </FieldShell>
      </section>

      <section className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_380px]">
        <div>
          {files.isPending ? (
            <LoadingBlock />
          ) : !files.data?.length ? (
            <EmptyBlock
              icon={FileSearch}
              title="No files found"
              description="Uploaded files will appear here after apps start using your presets."
            />
          ) : (
            <div className="grid gap-3">
              {files.data.map((file) => (
                <FileCard
                  key={file.id}
                  file={file}
                  selected={selected?.id === file.id}
                  onSelect={() => setSelected(file)}
                  onDelete={() => onDelete(file)}
                  deleting={remove.isPending}
                />
              ))}
            </div>
          )}
        </div>

        <aside className="rounded-3xl border border-default-200 bg-background p-5 shadow-sm xl:sticky xl:top-24 xl:self-start">
          {!selected ? (
            <div className="flex flex-col items-center gap-3 py-12 text-center">
              <Info className="h-8 w-8 text-default-400" />
              <div>
                <h2 className="font-semibold">Select a file</h2>
                <p className="mt-1 text-sm text-default-500">Metadata and upload logs will show here.</p>
              </div>
            </div>
          ) : (
            <FileDetails file={selected} logs={logs.data ?? []} logsLoading={logs.isPending} />
          )}
        </aside>
      </section>
    </div>
  );
}

function FileCard({
  file,
  selected,
  onSelect,
  onDelete,
  deleting,
}: {
  file: FileRecord;
  selected: boolean;
  onSelect: () => void;
  onDelete: () => void;
  deleting: boolean;
}) {
  return (
    <article className={`rounded-3xl border bg-background p-4 shadow-sm transition ${selected ? "border-accent ring-2 ring-accent/15" : "border-default-200 hover:border-accent/40"}`}>
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <button type="button" onClick={onSelect} className="flex min-w-0 flex-1 items-start gap-3 text-left">
          <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-accent/10 text-accent">
            <FolderOpen className="h-5 w-5" />
          </span>
          <span className="min-w-0">
            <span className="block truncate font-semibold">{file.original_name}</span>
            <span className="mt-1 block truncate font-mono text-xs text-default-400">{file.path}</span>
            <span className="mt-2 flex flex-wrap gap-2">
              <Chip size="sm" variant="soft">{file.mime_type}</Chip>
              <Chip size="sm" variant="soft" color={file.status === "uploaded" ? "success" : "default"}>{file.status}</Chip>
              <Chip size="sm" variant="soft">{formatBytes(file.size)}</Chip>
            </span>
          </span>
        </button>
        <div className="flex flex-wrap gap-2 md:justify-end">
          <Button size="sm" variant="tertiary" onPress={() => navigator.clipboard?.writeText(file.url)}>
            <Copy className="h-3.5 w-3.5" /> Copy URL
          </Button>
          <Button size="sm" variant="tertiary" onPress={() => window.open(file.url, "_blank", "noopener,noreferrer")}>
            <ExternalLink className="h-3.5 w-3.5" /> Open
          </Button>
          <Button size="sm" variant="danger-soft" onPress={onDelete} isPending={deleting}>
            <Trash2 className="h-3.5 w-3.5" /> Delete
          </Button>
        </div>
      </div>
    </article>
  );
}

function FileDetails({
  file,
  logs,
  logsLoading,
}: {
  file: FileRecord;
  logs: { id: string; event: string; status: string; message: string | null; created_at: string }[];
  logsLoading: boolean;
}) {
  return (
    <div className="flex flex-col gap-5">
      <div>
        <h2 className="truncate text-lg font-semibold">{file.original_name}</h2>
        <p className="mt-1 break-all font-mono text-xs text-default-400">{file.id}</p>
      </div>
      <dl className="grid gap-3 text-sm">
        <InfoRow label="Public URL" value={file.url} mono />
        <InfoRow label="Path" value={file.path} mono />
        <InfoRow label="Hash" value={file.hash} mono />
        <InfoRow label="Storage connection" value={file.storage_connection_id} mono />
        <InfoRow label="Created" value={new Date(file.created_at).toLocaleString()} />
        <InfoRow label="Updated" value={new Date(file.updated_at).toLocaleString()} />
      </dl>
      <div className="border-t border-default-100 pt-4">
        <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold">
          <CalendarDays className="h-4 w-4" /> Upload logs
        </h3>
        {logsLoading ? (
          <LoadingBlock />
        ) : !logs.length ? (
          <p className="text-sm text-default-500">No logs recorded for this file.</p>
        ) : (
          <div className="grid gap-2">
            {logs.map((log) => (
              <div key={log.id} className="rounded-2xl border border-default-100 bg-default-50 p-3 text-sm">
                <div className="flex items-center justify-between gap-3">
                  <span className="font-medium">{log.event}</span>
                  <Chip size="sm" variant="soft" color={log.status === "success" ? "success" : "default"}>{log.status}</Chip>
                </div>
                <p className="mt-1 text-xs text-default-500">{new Date(log.created_at).toLocaleString()}</p>
                {log.message && <p className="mt-2 text-sm text-default-600">{log.message}</p>}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function InfoRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="rounded-2xl border border-default-100 bg-default-50 p-3">
      <dt className="text-xs font-medium uppercase tracking-wide text-default-500">{label}</dt>
      <dd className={`mt-1 break-all text-default-800 ${mono ? "font-mono text-xs" : "text-sm"}`}>{value}</dd>
    </div>
  );
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}
