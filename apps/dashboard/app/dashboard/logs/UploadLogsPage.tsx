"use client";

import { Chip, Input } from "@heroui/react";
import { Activity, FileClock, Search } from "lucide-react";
import { useDeferredValue, useState } from "react";
import {
  EmptyBlock,
  FieldShell,
  LoadingBlock,
  NativeSelect,
  PageHeader,
} from "../../../components/PageUI";
import type { UploadLogFilters } from "../../../lib/api";
import { useProjects, useUploadLogs } from "../../../lib/queries";

export function UploadLogsPage() {
  const projects = useProjects();
  const [projectId, setProjectId] = useState("");
  const [event, setEvent] = useState("");
  const [status, setStatus] = useState("");
  const [search, setSearch] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const deferredSearch = useDeferredValue(search);
  const filters: UploadLogFilters = {
    project_id: projectId || undefined,
    event: event || undefined,
    status: status || undefined,
    search: deferredSearch || undefined,
    from: from ? new Date(`${from}T00:00:00`).toISOString() : undefined,
    to: to ? new Date(`${to}T23:59:59`).toISOString() : undefined,
    limit: 300,
  };
  const logs = useUploadLogs(filters);

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        icon={Activity}
        title="Upload logs"
        description="Trace uploads, duplicate decisions, and webhook delivery activity across projects."
      />

      <section className="grid gap-3 rounded-3xl border border-default-200 bg-background p-4 shadow-sm lg:grid-cols-[1.3fr_1fr_1fr_1fr_1fr_1fr]">
        <FieldShell label="Search">
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-default-400" />
            <Input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
              placeholder="Event, status, file ID"
            />
          </div>
        </FieldShell>
        <FieldShell label="Project">
          <NativeSelect
            value={projectId}
            onChange={(e) => setProjectId(e.target.value)}
          >
            <option value="">All projects</option>
            {projects.data?.map((project) => (
              <option key={project.id} value={project.id}>
                {project.name}
              </option>
            ))}
          </NativeSelect>
        </FieldShell>
        <FieldShell label="Event">
          <NativeSelect
            value={event}
            onChange={(e) => setEvent(e.target.value)}
          >
            <option value="">All events</option>
            <option value="file.uploaded">File uploaded</option>
            <option value="file.duplicate_detected">Duplicate detected</option>
            <option value="webhook.file.uploaded">Webhook: uploaded</option>
            <option value="webhook.file.deleted">Webhook: deleted</option>
            <option value="webhook.file.failed">Webhook: failed</option>
          </NativeSelect>
        </FieldShell>
        <FieldShell label="Status">
          <NativeSelect
            value={status}
            onChange={(e) => setStatus(e.target.value)}
          >
            <option value="">All statuses</option>
            <option value="success">Success</option>
            <option value="queued">Queued</option>
            <option value="rejected">Rejected</option>
            <option value="failed">Failed</option>
          </NativeSelect>
        </FieldShell>
        <FieldShell label="From">
          <Input
            type="date"
            value={from}
            onChange={(e) => setFrom(e.target.value)}
          />
        </FieldShell>
        <FieldShell label="To">
          <Input
            type="date"
            value={to}
            onChange={(e) => setTo(e.target.value)}
          />
        </FieldShell>
      </section>

      {logs.isPending ? (
        <LoadingBlock />
      ) : !logs.data?.length ? (
        <EmptyBlock
          icon={FileClock}
          title="No upload logs found"
          description="Try widening the filters or upload a file to generate activity."
        />
      ) : (
        <section className="grid gap-3">
          {logs.data.map((log) => {
            const project = projects.data?.find(
              (candidate) => candidate.id === log.project_id,
            );
            return (
              <article
                key={log.id}
                className="rounded-3xl border border-default-200 bg-background p-4 shadow-sm"
              >
                <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-medium">{log.event}</span>
                      <Chip
                        size="sm"
                        variant="soft"
                        color={statusColor(log.status)}
                      >
                        {log.status}
                      </Chip>
                    </div>
                    <p className="mt-1 text-sm text-default-500">
                      {project?.name ?? log.project_id}
                      {log.file_name ? ` · ${log.file_name}` : ""}
                    </p>
                    {log.message && (
                      <p className="mt-2 text-sm text-default-700">
                        {log.message}
                      </p>
                    )}
                  </div>
                  <time className="shrink-0 text-xs text-default-500">
                    {new Date(log.created_at).toLocaleString()}
                  </time>
                </div>
                <details className="mt-3 border-t border-default-100 pt-3">
                  <summary className="cursor-pointer text-xs font-medium text-default-500">
                    Technical details
                  </summary>
                  <pre className="mt-2 overflow-auto rounded-2xl bg-default-50 p-3 text-xs text-default-700">
                    {JSON.stringify(
                      {
                        id: log.id,
                        file_id: log.file_id,
                        metadata: log.metadata,
                      },
                      null,
                      2,
                    )}
                  </pre>
                </details>
              </article>
            );
          })}
        </section>
      )}
    </div>
  );
}

function statusColor(
  status: string,
): "success" | "danger" | "warning" | "default" {
  if (status === "success") return "success";
  if (status === "failed" || status === "rejected") return "danger";
  if (status === "queued") return "warning";
  return "default";
}
