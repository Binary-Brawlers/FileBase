"use client";

import { Card } from "@heroui/react";
import {
  Database,
  ChartNoAxesCombined,
  FolderOpen,
  HardDrive,
  KeyRound,
  Settings,
  type LucideIcon,
} from "lucide-react";
import Link from "next/link";
import { useAnalytics, useMe } from "../../lib/queries";
import { getToken } from "../../lib/auth";
import { useEffect, useMemo, useState } from "react";

type Stat = {
  label: string;
  value: string;
  hint: string;
  icon: LucideIcon;
  tone: "accent" | "primary" | "success" | "warning";
};

const QUICK_LINKS: {
  href: string;
  title: string;
  desc: string;
  icon: LucideIcon;
}[] = [
  {
    href: "/dashboard/analytics",
    title: "Analytics",
    desc: "Inspect upload volume, storage usage, and outcomes.",
    icon: ChartNoAxesCombined,
  },
  {
    href: "/dashboard/files",
    title: "Files",
    desc: "Browse uploads, copy URLs, and inspect logs.",
    icon: FolderOpen,
  },
  {
    href: "/dashboard/presets",
    title: "Upload presets",
    desc: "Define folders, size limits, and duplicate handling.",
    icon: Settings,
  },
  {
    href: "/dashboard/storage",
    title: "Storage connections",
    desc: "Add or rotate Local, FTP, and SFTP backends.",
    icon: Database,
  },
  {
    href: "/dashboard/api-keys",
    title: "API keys",
    desc: "Issue live or test keys for your apps.",
    icon: KeyRound,
  },
];

export default function DashboardHome() {
  const [token, setToken] = useState<string | null>(null);
  useEffect(() => setToken(getToken()), []);
  const me = useMe(token);
  const analyticsFilters = useMemo(() => lastDays(30), []);
  const analytics = useAnalytics(analyticsFilters);
  const stats: Stat[] = [
    {
      label: "Files",
      value: analytics.data ? formatNumber(analytics.data.totals.files) : "…",
      hint: "Uploads tracked",
      icon: FolderOpen,
      tone: "accent",
    },
    {
      label: "Storage used",
      value: analytics.data
        ? formatBytes(analytics.data.totals.storage_bytes)
        : "…",
      hint: "Across all backends",
      icon: HardDrive,
      tone: "primary",
    },
    {
      label: "30-day uploads",
      value: analytics.data ? formatNumber(analytics.data.period.uploads) : "…",
      hint: analytics.data
        ? formatBytes(analytics.data.period.uploaded_bytes)
        : "Recent volume",
      icon: ChartNoAxesCombined,
      tone: "success",
    },
    {
      label: "Folders",
      value: analytics.data ? formatNumber(analytics.data.totals.folders) : "…",
      hint: `${analytics.data?.totals.projects ?? "…"} projects in scope`,
      icon: Settings,
      tone: "warning",
    },
  ];

  return (
    <div className="flex flex-col gap-8">
      <div className="flex flex-col gap-1">
        <h1 className="text-3xl font-semibold tracking-tight">
          Welcome back{me.data?.name ? `, ${me.data.name.split(" ")[0]}` : ""}
        </h1>
        <p className="text-default-500">
          Here&apos;s a snapshot of your FileBase install.
        </p>
      </div>

      <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {stats.map((s) => (
          <StatCard key={s.label} stat={s} />
        ))}
      </section>

      <section className="grid gap-6 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <Card.Header>
            <Card.Title>Getting started</Card.Title>
            <Card.Description>
              A few things you can do from here.
            </Card.Description>
          </Card.Header>
          <Card.Content className="grid gap-3 sm:grid-cols-3">
            {QUICK_LINKS.map((q) => {
              const Icon = q.icon;
              return (
                <Link
                  key={q.href}
                  href={q.href}
                  className="group flex flex-col gap-2 rounded-xl border border-default-200 p-4 transition hover:border-accent hover:bg-accent/5"
                >
                  <span
                    className="flex h-9 w-9 items-center justify-center rounded-lg bg-default-100 text-default-700 group-hover:bg-accent/15 group-hover:text-accent"
                    aria-hidden
                  >
                    <Icon className="h-5 w-5" />
                  </span>
                  <span className="font-medium">{q.title}</span>
                  <span className="text-xs text-default-500">{q.desc}</span>
                </Link>
              );
            })}
          </Card.Content>
        </Card>

        <Card>
          <Card.Header>
            <Card.Title>Install info</Card.Title>
            <Card.Description>Your active account.</Card.Description>
          </Card.Header>
          <Card.Content className="gap-4">
            <InfoRow label="Signed in as" value={me.data?.email ?? "—"} />
            <InfoRow label="Name" value={me.data?.name ?? "—"} />
            <Link
              href="/dashboard/api-keys"
              className="inline-flex w-full items-center justify-center rounded-lg border border-default-300 px-4 py-2 text-sm font-medium hover:bg-default-100"
            >
              Manage API keys
            </Link>
          </Card.Content>
        </Card>
      </section>
    </div>
  );
}

function lastDays(days: number) {
  const to = new Date();
  const from = new Date(to);
  from.setUTCDate(from.getUTCDate() - (days - 1));
  from.setUTCHours(0, 0, 0, 0);
  to.setUTCHours(23, 59, 59, 999);
  return { from: from.toISOString(), to: to.toISOString() };
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

function StatCard({ stat }: { stat: Stat }) {
  const toneClasses: Record<Stat["tone"], string> = {
    accent: "bg-accent/15 text-accent",
    primary: "bg-primary/15 text-primary",
    success: "bg-success/15 text-success",
    warning: "bg-warning/15 text-warning",
  };
  const Icon = stat.icon;
  return (
    <Card>
      <Card.Content className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <span className="text-xs uppercase tracking-wide text-default-500">
            {stat.label}
          </span>
          <span className="text-3xl font-semibold tracking-tight">
            {stat.value}
          </span>
          <span className="text-xs text-default-500">{stat.hint}</span>
        </div>
        <span
          className={
            "flex h-10 w-10 items-center justify-center rounded-lg " +
            toneClasses[stat.tone]
          }
          aria-hidden
        >
          <Icon className="h-5 w-5" />
        </span>
      </Card.Content>
    </Card>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3 text-sm">
      <span className="text-default-500">{label}</span>
      <span className="font-medium truncate">{value}</span>
    </div>
  );
}
