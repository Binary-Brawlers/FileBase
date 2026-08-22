"use client";

import { Button } from "@heroui/react";
import { ChartNoAxesCombined } from "lucide-react";
import { useMemo, useState } from "react";
import {
  Alert,
  FieldShell,
  LoadingBlock,
  NativeSelect,
  PageHeader,
} from "../../../components/PageUI";
import type { AnalyticsBreakdown, AnalyticsFilters } from "../../../lib/api";
import { useAnalytics, useProjects } from "../../../lib/queries";

type RangeDays = 7 | 30 | 90;

export function AnalyticsPage() {
  const projects = useProjects();
  const [projectId, setProjectId] = useState("");
  const [rangeDays, setRangeDays] = useState<RangeDays>(30);
  const filters = useMemo(
    () => analyticsRange(projectId, rangeDays),
    [projectId, rangeDays],
  );
  const analytics = useAnalytics(filters);
  const data = analytics.data;

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        icon={ChartNoAxesCombined}
        title="Analytics"
        description="Monitor upload volume, storage usage, content mix, and operational outcomes."
      />

      <section className="flex flex-wrap items-end gap-4 border-y border-default-200 py-4">
        <FieldShell label="Project">
          <NativeSelect
            value={projectId}
            onChange={(event) => setProjectId(event.target.value)}
          >
            <option value="">All projects</option>
            {projects.data?.map((project) => (
              <option key={project.id} value={project.id}>
                {project.name}
              </option>
            ))}
          </NativeSelect>
        </FieldShell>
        <div className="flex flex-col gap-1.5">
          <span className="text-sm font-medium text-default-700">Window</span>
          <div className="flex gap-1 rounded-xl bg-default-100 p-1">
            {([7, 30, 90] as const).map((days) => (
              <Button
                key={days}
                size="sm"
                variant={rangeDays === days ? "primary" : "tertiary"}
                onPress={() => setRangeDays(days)}
              >
                {days} days
              </Button>
            ))}
          </div>
        </div>
        <p className="ml-auto text-xs text-default-500">
          Totals are current. Trends and breakdowns use the selected window.
        </p>
      </section>

      {analytics.isError && (
        <Alert message="Analytics could not be loaded. Check the API and try again." />
      )}

      {analytics.isPending || !data ? (
        <LoadingBlock />
      ) : (
        <>
          <section className="grid border-y border-default-200 sm:grid-cols-2 xl:grid-cols-4 xl:divide-x xl:divide-default-200">
            <Metric
              label="Stored files"
              value={formatNumber(data.totals.files)}
              hint={formatBytes(data.totals.storage_bytes)}
            />
            <Metric
              label="Uploads in window"
              value={formatNumber(data.period.uploads)}
              hint={formatBytes(data.period.uploaded_bytes)}
            />
            <Metric
              label="Success rate"
              value={
                data.period.success_rate === null
                  ? "—"
                  : `${data.period.success_rate.toFixed(1)}%`
              }
              hint={`${data.period.failure_events} failed or rejected`}
            />
            <Metric
              label="Folder paths"
              value={formatNumber(data.totals.folders)}
              hint={`${data.totals.projects} projects in scope`}
            />
          </section>

          <UploadTrend points={data.trend} />

          <section className="grid gap-10 border-t border-default-200 pt-8 lg:grid-cols-2">
            <BreakdownList
              title="Content types"
              description="Uploads grouped by MIME type."
              rows={data.mime_types}
            />
            <BreakdownList
              title="Storage destinations"
              description="Upload volume by storage adapter."
              rows={data.storage_types}
            />
          </section>

          <section className="grid gap-10 border-t border-default-200 pt-8 lg:grid-cols-[1.4fr_0.6fr]">
            <BreakdownList
              title="Top folders"
              description="Folder paths receiving the most uploads."
              rows={data.folders}
            />
            <OutcomeList
              outcomes={data.outcomes}
              duplicateEvents={data.period.duplicate_events}
            />
          </section>
        </>
      )}
    </div>
  );
}

function Metric({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint: string;
}) {
  return (
    <div className="px-1 py-5 sm:px-5 xl:first:pl-0 xl:last:pr-0">
      <p className="text-xs font-medium uppercase tracking-wide text-default-500">
        {label}
      </p>
      <p className="mt-2 text-3xl font-semibold tracking-tight">{value}</p>
      <p className="mt-1 text-sm text-default-500">{hint}</p>
    </div>
  );
}

function UploadTrend({
  points,
}: {
  points: { date: string; uploads: number; bytes: number }[];
}) {
  const maxUploads = Math.max(1, ...points.map((point) => point.uploads));
  const totalUploads = points.reduce((sum, point) => sum + point.uploads, 0);
  return (
    <section>
      <div className="flex items-end justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold">Daily uploads</h2>
          <p className="mt-1 text-sm text-default-500">
            {formatNumber(totalUploads)} uploads across {points.length} days.
          </p>
        </div>
        <p className="text-xs text-default-500">Hover bars for daily totals</p>
      </div>
      <div
        className="mt-6 grid h-64 items-end gap-1 border-b border-default-200"
        style={{
          gridTemplateColumns: `repeat(${Math.max(points.length, 1)}, minmax(3px, 1fr))`,
        }}
      >
        {points.map((point) => {
          const height =
            point.uploads === 0
              ? 2
              : Math.max(6, (point.uploads / maxUploads) * 100);
          return (
            <div
              key={point.date}
              className="group relative flex h-full items-end"
              title={`${point.date}: ${point.uploads} uploads, ${formatBytes(point.bytes)}`}
            >
              <div
                className="w-full rounded-t-sm bg-accent/65 transition group-hover:bg-accent"
                style={{ height: `${height}%` }}
              />
            </div>
          );
        })}
      </div>
      {points.length > 0 && (
        <div className="mt-2 flex justify-between text-xs text-default-500">
          <time>{formatDate(points[0].date)}</time>
          <time>{formatDate(points[points.length - 1].date)}</time>
        </div>
      )}
    </section>
  );
}

function BreakdownList({
  title,
  description,
  rows,
}: {
  title: string;
  description: string;
  rows: AnalyticsBreakdown[];
}) {
  const maxCount = Math.max(1, ...rows.map((row) => row.count));
  return (
    <div>
      <h2 className="text-lg font-semibold">{title}</h2>
      <p className="mt-1 text-sm text-default-500">{description}</p>
      {!rows.length ? (
        <p className="mt-6 text-sm text-default-500">
          No uploads in this window.
        </p>
      ) : (
        <div className="mt-6 divide-y divide-default-100">
          {rows.map((row) => (
            <div key={row.key} className="group py-3">
              <div className="flex items-center justify-between gap-4 text-sm">
                <span className="min-w-0 truncate font-medium">{row.key}</span>
                <span className="shrink-0 text-default-500">
                  {formatNumber(row.count)} · {formatBytes(row.bytes)}
                </span>
              </div>
              <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-default-100">
                <div
                  className="h-full rounded-full bg-accent/60 transition group-hover:bg-accent"
                  style={{
                    width: `${Math.max(3, (row.count / maxCount) * 100)}%`,
                  }}
                />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function OutcomeList({
  outcomes,
  duplicateEvents,
}: {
  outcomes: { status: string; count: number }[];
  duplicateEvents: number;
}) {
  return (
    <div>
      <h2 className="text-lg font-semibold">Operational signals</h2>
      <p className="mt-1 text-sm text-default-500">
        Upload and webhook log outcomes in this window.
      </p>
      <dl className="mt-6 divide-y divide-default-100">
        {outcomes.map((outcome) => (
          <div key={outcome.status} className="flex justify-between gap-4 py-3">
            <dt className="capitalize text-default-600">{outcome.status}</dt>
            <dd className="font-medium">{formatNumber(outcome.count)}</dd>
          </div>
        ))}
        <div className="flex justify-between gap-4 py-3">
          <dt className="text-default-600">Duplicate events</dt>
          <dd className="font-medium">{formatNumber(duplicateEvents)}</dd>
        </div>
      </dl>
    </div>
  );
}

function analyticsRange(projectId: string, days: RangeDays): AnalyticsFilters {
  const to = new Date();
  const from = new Date(to);
  from.setUTCDate(from.getUTCDate() - (days - 1));
  from.setUTCHours(0, 0, 0, 0);
  to.setUTCHours(23, 59, 59, 999);
  return {
    project_id: projectId || undefined,
    from: from.toISOString(),
    to: to.toISOString(),
  };
}

function formatNumber(value: number) {
  return new Intl.NumberFormat().format(value);
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 ** 2) return `${(value / 1024).toFixed(1)} KB`;
  if (value < 1024 ** 3) return `${(value / 1024 ** 2).toFixed(1)} MB`;
  return `${(value / 1024 ** 3).toFixed(1)} GB`;
}

function formatDate(value: string) {
  return new Date(`${value}T00:00:00Z`).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}
