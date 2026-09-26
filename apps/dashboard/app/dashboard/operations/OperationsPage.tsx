"use client";

import { Button, Chip } from "@heroui/react";
import {
  Activity,
  Database,
  Film,
  HardDrive,
  RefreshCw,
  Trash2,
  Wrench,
} from "lucide-react";
import { useState } from "react";
import {
  Alert,
  EmptyBlock,
  LoadingBlock,
  PageHeader,
} from "../../../components/PageUI";
import { ApiError, type CleanupScope } from "../../../lib/api";
import {
  useDeleteJob,
  useDiagnostics,
  useFailedJobs,
  useRetryJob,
  useRunCleanup,
  useUpgradeCheck,
} from "../../../lib/queries";

function formatBytes(bytes: number | null | undefined) {
  if (bytes === null || bytes === undefined) return "-";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(1)} ${units[unit]}`;
}

function formatUptime(seconds: number) {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

function StatusDot({ status }: { status: "ok" | "warning" | "error" }) {
  const color =
    status === "ok"
      ? "bg-success"
      : status === "warning"
        ? "bg-warning"
        : "bg-danger";
  return (
    <span
      className={`inline-block h-2.5 w-2.5 rounded-full ${color}`}
      aria-label={status}
    />
  );
}

function MaintenanceCard({ children }: { children: React.ReactNode }) {
  return (
    <div className="rounded-2xl border border-default-200 bg-background p-5 shadow-sm">
      {children}
    </div>
  );
}

export function OperationsPage() {
  const diagnostics = useDiagnostics();
  const upgrade = useUpgradeCheck();
  const jobs = useFailedJobs(50);
  const retry = useRetryJob();
  const removeJob = useDeleteJob();
  const cleanup = useRunCleanup();
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function runCleanup(scope: CleanupScope, label: string) {
    if (!confirm(`${label}? This cannot be undone.`)) return;
    setError(null);
    setNotice(null);
    try {
      const result = await cleanup.mutateAsync({ scope });
      setNotice(
        `${label}: removed ${result.removedCount} item(s), reclaimed ${formatBytes(
          result.removedBytes,
        )}.`,
      );
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Cleanup failed.");
    }
  }

  async function onRetry(jobId: string) {
    setError(null);
    setNotice(null);
    try {
      await retry.mutateAsync(jobId);
      setNotice("Job requeued for processing.");
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Retry failed.");
    }
  }

  async function onDelete(jobId: string) {
    if (!confirm("Delete this failed job permanently?")) return;
    setError(null);
    setNotice(null);
    try {
      await removeJob.mutateAsync(jobId);
      setNotice("Failed job deleted.");
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Delete failed.");
    }
  }

  const data = diagnostics.data;

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        icon={Wrench}
        title="Operations"
        description="Diagnostics, maintenance controls, and failed job recovery for this installation."
        action={
          <Button
            variant="tertiary"
            onPress={() => diagnostics.refetch()}
            isPending={diagnostics.isFetching}
          >
            <RefreshCw /> Refresh
          </Button>
        }
      />

      {error && <Alert message={error} />}
      {notice && (
        <div
          role="status"
          className="rounded-lg border border-success/30 bg-success/10 px-3 py-2 text-sm text-success"
        >
          {notice}
        </div>
      )}

      {diagnostics.isPending ? (
        <LoadingBlock />
      ) : diagnostics.isError || !data ? (
        <Alert message="Unable to load diagnostics. Operator access is required." />
      ) : (
        <>
          <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
            <MaintenanceCard>
              <div className="flex items-center justify-between">
                <h2 className="flex items-center gap-2 text-sm font-semibold">
                  <Database className="h-4 w-4" /> Database
                </h2>
                <StatusDot status={data.database.ok ? "ok" : "error"} />
              </div>
              <dl className="mt-3 grid grid-cols-2 gap-2 text-xs text-default-600">
                <div>
                  <dt className="text-default-500">Size</dt>
                  <dd>{formatBytes(data.database.sizeBytes)}</dd>
                </div>
                <div>
                  <dt className="text-default-500">Users</dt>
                  <dd>{data.database.users}</dd>
                </div>
                <div>
                  <dt className="text-default-500">Projects</dt>
                  <dd>{data.database.projects}</dd>
                </div>
                <div>
                  <dt className="text-default-500">Files</dt>
                  <dd>
                    {data.database.files} /{" "}
                    {formatBytes(data.database.fileBytes)}
                  </dd>
                </div>
              </dl>
            </MaintenanceCard>

            <MaintenanceCard>
              <div className="flex items-center justify-between">
                <h2 className="flex items-center gap-2 text-sm font-semibold">
                  <Activity className="h-4 w-4" /> Redis queue
                </h2>
                <StatusDot status={data.redis.ok ? "ok" : "warning"} />
              </div>
              <dl className="mt-3 grid grid-cols-3 gap-2 text-xs text-default-600">
                <div>
                  <dt className="text-default-500">Pending</dt>
                  <dd>{data.redis.pendingJobs ?? "-"}</dd>
                </div>
                <div>
                  <dt className="text-default-500">Processing</dt>
                  <dd>{data.redis.processingJobs ?? "-"}</dd>
                </div>
                <div>
                  <dt className="text-default-500">Failed</dt>
                  <dd>{data.redis.failedJobs ?? "-"}</dd>
                </div>
              </dl>
            </MaintenanceCard>

            <MaintenanceCard>
              <div className="flex items-center justify-between">
                <h2 className="flex items-center gap-2 text-sm font-semibold">
                  <HardDrive className="h-4 w-4" /> Storage
                </h2>
              </div>
              <dl className="mt-3 grid grid-cols-2 gap-2 text-xs text-default-600">
                <div className="col-span-2 truncate">
                  <dt className="text-default-500">Local path</dt>
                  <dd className="truncate font-mono">
                    {data.storage.localPath}
                  </dd>
                </div>
                <div>
                  <dt className="text-default-500">Local files</dt>
                  <dd>
                    {data.storage.localFiles} /{" "}
                    {formatBytes(data.storage.localBytes)}
                  </dd>
                </div>
                <div>
                  <dt className="text-default-500">Temp files</dt>
                  <dd>
                    {data.storage.tempFiles} /{" "}
                    {formatBytes(data.storage.tempBytes)}
                  </dd>
                </div>
                <div>
                  <dt className="text-default-500">Chunk uploads</dt>
                  <dd>
                    {data.storage.chunkRows} /{" "}
                    {formatBytes(data.storage.chunkBytes)}
                  </dd>
                </div>
                <div>
                  <dt className="text-default-500">Open sessions</dt>
                  <dd>{data.storage.pendingUploadSessions}</dd>
                </div>
                <div>
                  <dt className="text-default-500">Transform cache</dt>
                  <dd>
                    {data.storage.transformCacheFiles} /{" "}
                    {formatBytes(data.storage.transformCacheBytes)}
                  </dd>
                </div>
              </dl>
            </MaintenanceCard>

            <MaintenanceCard>
              <div className="flex items-center justify-between">
                <h2 className="flex items-center gap-2 text-sm font-semibold">
                  <Film className="h-4 w-4" /> Media tooling
                </h2>
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                <Chip
                  size="sm"
                  variant="soft"
                  color={data.media.ffprobeAvailable ? "success" : "default"}
                >
                  ffprobe {data.media.ffprobeAvailable ? "ready" : "missing"}
                </Chip>
                <Chip
                  size="sm"
                  variant="soft"
                  color={data.media.ffmpegAvailable ? "success" : "default"}
                >
                  ffmpeg {data.media.ffmpegAvailable ? "ready" : "missing"}
                </Chip>
              </div>
            </MaintenanceCard>

            <MaintenanceCard>
              <h2 className="text-sm font-semibold">Runtime</h2>
              <dl className="mt-3 grid grid-cols-2 gap-2 text-xs text-default-600">
                <div>
                  <dt className="text-default-500">Version</dt>
                  <dd>{data.version}</dd>
                </div>
                <div>
                  <dt className="text-default-500">Uptime</dt>
                  <dd>{formatUptime(data.uptimeSeconds)}</dd>
                </div>
                <div>
                  <dt className="text-default-500">Max upload</dt>
                  <dd>{formatBytes(data.limits.maxUploadSize)}</dd>
                </div>
                <div>
                  <dt className="text-default-500">Chunk size</dt>
                  <dd>{formatBytes(data.limits.uploadChunkSize)}</dd>
                </div>
                <div>
                  <dt className="text-default-500">Auth limit</dt>
                  <dd>{data.limits.authRateLimitPerMinute}/min</dd>
                </div>
                <div>
                  <dt className="text-default-500">Upload limit</dt>
                  <dd>{data.limits.uploadRateLimitPerMinute}/min</dd>
                </div>
              </dl>
            </MaintenanceCard>
          </section>

          <section className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-lg font-semibold">Upgrade safety</h2>
                <p className="text-sm text-default-500">
                  Pre-flight checks to run before pulling a new release.
                </p>
              </div>
              {upgrade.data && (
                <Chip
                  size="sm"
                  variant="soft"
                  color={upgrade.data.upgradeSafe ? "success" : "danger"}
                >
                  {upgrade.data.upgradeSafe ? "Ready" : "Attention needed"}
                </Chip>
              )}
            </div>
            {upgrade.isPending ? (
              <LoadingBlock />
            ) : upgrade.isError || !upgrade.data ? (
              <Alert message="Unable to run upgrade checks. Operator access is required." />
            ) : (
              <>
                <p className="text-sm text-default-500">
                  Version {upgrade.data.currentVersion} ·{" "}
                  {upgrade.data.appliedMigrations} migrations applied ·{" "}
                  {upgrade.data.pendingMigrations} pending
                </p>
                <ul className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
                  {upgrade.data.checks.map((check) => (
                    <li
                      key={check.name}
                      className="rounded-2xl border border-default-200 bg-background p-3 shadow-sm"
                    >
                      <div className="flex items-center gap-2">
                        <StatusDot status={check.status} />
                        <span className="text-sm font-medium">
                          {check.name}
                        </span>
                      </div>
                      <p className="mt-1 text-xs text-default-500">
                        {check.message}
                      </p>
                    </li>
                  ))}
                </ul>
              </>
            )}
          </section>

          <section className="flex flex-col gap-3">
            <div>
              <h2 className="text-lg font-semibold">Maintenance controls</h2>
              <p className="text-sm text-default-500">
                Remove stale data and reclaim disk space. Actions are recorded
                in the audit log.
              </p>
            </div>
            <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
              <MaintenanceCard>
                <h3 className="text-sm font-medium">Stale temp files</h3>
                <p className="mt-1 text-xs text-default-500">
                  Uploaded temp files older than one hour.
                </p>
                <Button
                  className="mt-3"
                  size="sm"
                  variant="tertiary"
                  onPress={() =>
                    runCleanup("temp", "Clean up stale temp files")
                  }
                  isPending={cleanup.isPending}
                >
                  Run cleanup
                </Button>
              </MaintenanceCard>
              <MaintenanceCard>
                <h3 className="text-sm font-medium">Expired sessions</h3>
                <p className="mt-1 text-xs text-default-500">
                  Unused upload sessions past their expiry.
                </p>
                <Button
                  className="mt-3"
                  size="sm"
                  variant="tertiary"
                  onPress={() =>
                    runCleanup("sessions", "Clean up expired sessions")
                  }
                  isPending={cleanup.isPending}
                >
                  Run cleanup
                </Button>
              </MaintenanceCard>
              <MaintenanceCard>
                <h3 className="text-sm font-medium">Orphaned chunks</h3>
                <p className="mt-1 text-xs text-default-500">
                  Chunk files and directories with no session.
                </p>
                <Button
                  className="mt-3"
                  size="sm"
                  variant="tertiary"
                  onPress={() =>
                    runCleanup("chunks", "Clean up orphaned chunks")
                  }
                  isPending={cleanup.isPending}
                >
                  Run cleanup
                </Button>
              </MaintenanceCard>
              <MaintenanceCard>
                <h3 className="text-sm font-medium">Transform cache</h3>
                <p className="mt-1 text-xs text-default-500">
                  Cached dynamic image transformations.
                </p>
                <Button
                  className="mt-3"
                  size="sm"
                  variant="tertiary"
                  onPress={() =>
                    runCleanup("transform_cache", "Clean up transform cache")
                  }
                  isPending={cleanup.isPending}
                >
                  Run cleanup
                </Button>
              </MaintenanceCard>
              <MaintenanceCard>
                <h3 className="text-sm font-medium">Audit log retention</h3>
                <p className="mt-1 text-xs text-default-500">
                  Delete audit entries older than 90 days.
                </p>
                <Button
                  className="mt-3"
                  size="sm"
                  variant="tertiary"
                  onPress={() => runCleanup("audit_logs", "Prune audit logs")}
                  isPending={cleanup.isPending}
                >
                  Run cleanup
                </Button>
              </MaintenanceCard>
            </div>
          </section>

          <section className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-lg font-semibold">Failed jobs</h2>
                <p className="text-sm text-default-500">
                  Jobs that exhausted their retry budget.
                </p>
              </div>
              <Button
                size="sm"
                variant="tertiary"
                onPress={() => jobs.refetch()}
                isPending={jobs.isFetching}
              >
                <RefreshCw /> Refresh
              </Button>
            </div>
            {jobs.isPending ? (
              <LoadingBlock />
            ) : !jobs.data?.length ? (
              <EmptyBlock
                icon={Wrench}
                title="No failed jobs"
                description="Background jobs that fail after retries will show up here for recovery."
              />
            ) : (
              <ul className="flex flex-col gap-3">
                {jobs.data.map((failed) => (
                  <li
                    key={failed.job.id}
                    className="rounded-2xl border border-default-200 bg-background p-4 shadow-sm"
                  >
                    <div className="flex flex-wrap items-center gap-2">
                      <Chip size="sm" variant="soft" color="danger">
                        {failed.job.kind}
                      </Chip>
                      <span className="font-mono text-xs text-default-500">
                        {failed.job.id}
                      </span>
                      <span className="text-xs text-default-500">
                        attempts {failed.job.attempts}/{failed.job.max_attempts}{" "}
                        · {new Date(failed.failed_at).toLocaleString()}
                      </span>
                    </div>
                    <p className="mt-2 break-words text-xs text-danger">
                      {failed.error}
                    </p>
                    <div className="mt-3 flex gap-2">
                      <Button
                        size="sm"
                        variant="primary"
                        onPress={() => onRetry(failed.job.id)}
                        isPending={retry.isPending}
                      >
                        Retry
                      </Button>
                      <Button
                        size="sm"
                        variant="danger-soft"
                        onPress={() => onDelete(failed.job.id)}
                        isPending={removeJob.isPending}
                      >
                        <Trash2 /> Delete
                      </Button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </>
      )}
    </div>
  );
}
