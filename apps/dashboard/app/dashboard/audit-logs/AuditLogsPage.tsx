"use client";

import { Chip, Input, Label, TextField } from "@heroui/react";
import { History } from "lucide-react";
import { useState } from "react";
import {
  Alert,
  EmptyBlock,
  FieldShell,
  LoadingBlock,
  NativeSelect,
  PageHeader,
} from "../../../components/PageUI";
import type { AuditLogFilters } from "../../../lib/api";
import { useAuditLogs, useProjects } from "../../../lib/queries";

function formatTimestamp(value: string) {
  return new Date(value).toLocaleString();
}

export function AuditLogsPage() {
  const projects = useProjects();
  const [projectId, setProjectId] = useState("");
  const [action, setAction] = useState("");
  const [status, setStatus] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");

  const filters: AuditLogFilters = {
    project_id: projectId || undefined,
    action: action.trim() || undefined,
    status: status || undefined,
    from: from ? new Date(from).toISOString() : undefined,
    to: to ? new Date(to).toISOString() : undefined,
    limit: 200,
  };
  const logs = useAuditLogs(filters);

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        icon={History}
        title="Audit logs"
        description="Persistent record of administrative and API activity across your installation."
      />

      <section className="grid gap-4 rounded-3xl border border-default-200 bg-background p-5 shadow-sm sm:grid-cols-2 xl:grid-cols-4">
        <FieldShell label="Project">
          <NativeSelect
            value={projectId}
            onChange={(event) => setProjectId(event.target.value)}
          >
            <option value="">All accessible projects</option>
            {projects.data?.map((project) => (
              <option key={project.id} value={project.id}>
                {project.name}
              </option>
            ))}
          </NativeSelect>
        </FieldShell>
        <FieldShell label="Action" hint="Exact match, e.g. auth.login_failed">
          <Input
            value={action}
            onChange={(event) => setAction(event.target.value)}
            placeholder="auth.login_succeeded"
          />
        </FieldShell>
        <FieldShell label="Status">
          <NativeSelect
            value={status}
            onChange={(event) => setStatus(event.target.value)}
          >
            <option value="">Any status</option>
            <option value="success">success</option>
            <option value="failure">failure</option>
          </NativeSelect>
        </FieldShell>
        <div className="grid grid-cols-2 gap-3">
          <TextField>
            <Label>From</Label>
            <Input
              type="datetime-local"
              value={from}
              onChange={(event) => setFrom(event.target.value)}
            />
          </TextField>
          <TextField>
            <Label>To</Label>
            <Input
              type="datetime-local"
              value={to}
              onChange={(event) => setTo(event.target.value)}
            />
          </TextField>
        </div>
      </section>

      {logs.isError ? (
        <Alert message="Unable to load audit logs. Verify your session and try again." />
      ) : logs.isPending ? (
        <LoadingBlock />
      ) : !logs.data?.length ? (
        <EmptyBlock
          icon={History}
          title="No audit events found"
          description="Activity from sign-ins, API keys, uploads, webhooks, and maintenance will appear here."
        />
      ) : (
        <ul className="flex flex-col gap-3">
          {logs.data.map((log) => (
            <li
              key={log.id}
              className="rounded-2xl border border-default-200 bg-background p-4 shadow-sm"
            >
              <div className="flex flex-wrap items-center gap-2">
                <Chip
                  size="sm"
                  variant="soft"
                  color={log.status === "success" ? "success" : "danger"}
                >
                  {log.status}
                </Chip>
                <span className="font-mono text-sm font-medium">
                  {log.action}
                </span>
                <span className="text-xs text-default-500">
                  {formatTimestamp(log.created_at)}
                </span>
              </div>
              <div className="mt-2 grid gap-1 text-xs text-default-600 sm:grid-cols-2">
                <span>
                  Actor: {log.actor_type}
                  {log.actor_email
                    ? ` (${log.actor_email})`
                    : log.actor_id
                      ? ` (${log.actor_id})`
                      : ""}
                </span>
                <span>Project: {log.project_id ?? "-"}</span>
                <span>
                  Resource: {log.resource_type ?? "-"}
                  {log.resource_id ? ` (${log.resource_id})` : ""}
                </span>
                <span>IP: {log.ip_address ?? "-"}</span>
              </div>
              {Object.keys(log.metadata ?? {}).length > 0 && (
                <details className="mt-2 text-xs text-default-500">
                  <summary className="cursor-pointer select-none">
                    Metadata
                  </summary>
                  <pre className="mt-1 overflow-x-auto rounded-lg bg-default-100 p-2">
                    {JSON.stringify(log.metadata, null, 2)}
                  </pre>
                </details>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
