"use client";

import { Button, Separator, Spinner } from "@heroui/react";
import {
  KeyRound,
  LayoutDashboard,
  Database,
  FolderOpen,
  Boxes,
  Settings,
  Webhook,
  type LucideIcon,
} from "lucide-react";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useState, type ReactNode } from "react";
import { Brand } from "../../components/Brand";
import { ThemeToggle } from "../../components/ThemeToggle";
import { clearToken, getToken } from "../../lib/auth";
import { useLogout, useMe } from "../../lib/queries";

type NavItem = { key: string; label: string; icon: LucideIcon };

const NAV: NavItem[] = [
  { key: "/dashboard", label: "Overview", icon: LayoutDashboard },
  { key: "/dashboard/projects", label: "Projects", icon: Boxes },
  { key: "/dashboard/files", label: "Files", icon: FolderOpen },
  { key: "/dashboard/presets", label: "Upload presets", icon: Settings },
  { key: "/dashboard/storage", label: "Storage connections", icon: Database },
  { key: "/dashboard/api-keys", label: "API keys", icon: KeyRound },
  { key: "/dashboard/webhooks", label: "Webhooks", icon: Webhook },
];

export function DashboardShell({ children }: { children: ReactNode }) {
  const router = useRouter();
  const pathname = usePathname();
  const [token, setTokenState] = useState<string | null>(null);
  const [mounted, setMounted] = useState(false);
  const me = useMe(token);
  const logout = useLogout();

  useEffect(() => {
    setMounted(true);
    const t = getToken();
    if (!t) {
      router.replace("/login");
      return;
    }
    setTokenState(t);
  }, [router]);

  useEffect(() => {
    if (me.isError) {
      clearToken();
      router.replace("/login");
    }
  }, [me.isError, router]);

  async function onLogout() {
    await logout.mutateAsync().catch(() => undefined);
    clearToken();
    router.replace("/login");
  }

  if (!mounted || me.isPending) {
    return (
      <div className="min-h-screen grid place-content-center gap-3 text-center">
        <Spinner />
        <span className="text-sm text-default-500">Loading...</span>
      </div>
    );
  }

  const userInitial = (me.data?.name || me.data?.email || "?")
    .charAt(0)
    .toUpperCase();
  const currentLabel =
    NAV.find((n) => n.key === pathname)?.label ?? "Dashboard";

  return (
    <div className="grid min-h-screen grid-cols-1 lg:grid-cols-[260px_1fr] bg-default-50">
      <aside className="hidden lg:flex flex-col gap-4 border-r border-default-200 bg-background p-5">
        <Brand tagline="Admin console" />
        <Separator />

        <nav aria-label="Primary" className="flex flex-col gap-1">
          {NAV.map((item) => {
            const active = pathname === item.key;
            const Icon = item.icon;
            return (
              <Link
                key={item.key}
                href={item.key}
                className={
                  "flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition " +
                  (active
                    ? "bg-accent/10 text-accent font-medium"
                    : "text-default-700 hover:bg-default-100")
                }
              >
                <Icon className="h-4 w-4 shrink-0" aria-hidden />
                <span>{item.label}</span>
              </Link>
            );
          })}
        </nav>

        <div className="flex-1" />
        <Separator />

        <div className="flex items-center gap-3">
          <span
            className="flex h-9 w-9 items-center justify-center rounded-full bg-accent/15 text-accent font-semibold"
            aria-hidden
          >
            {userInitial}
          </span>
          <div className="flex flex-col min-w-0 flex-1">
            <span className="text-sm font-medium truncate">
              {me.data?.name || "Admin"}
            </span>
            <span className="text-xs text-default-500 truncate">
              {me.data?.email}
            </span>
          </div>
        </div>

        <div className="flex items-center justify-between gap-2">
          <ThemeToggle />
          <Button
            size="sm"
            variant="danger-soft"
            onPress={onLogout}
            isPending={logout.isPending}
          >
            Sign out
          </Button>
        </div>
      </aside>

      <div className="flex flex-col min-w-0">
        <header className="sticky top-0 z-10 flex items-center justify-between gap-4 border-b border-default-200 bg-background/80 px-6 py-3 backdrop-blur lg:px-10">
          <div className="flex items-center gap-3 min-w-0">
            <div className="lg:hidden">
              <Brand showWordmark={false} />
            </div>
            <h1 className="text-sm font-medium text-default-600 truncate">
              {currentLabel}
            </h1>
          </div>
          <div className="flex items-center gap-2 lg:hidden">
            <ThemeToggle />
            <Button size="sm" variant="tertiary" onPress={onLogout}>
              Sign out
            </Button>
          </div>
        </header>
        <main className="p-6 lg:p-10 max-w-7xl w-full">{children}</main>
      </div>
    </div>
  );
}
