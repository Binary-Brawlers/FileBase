"use client";

import { Modal, Spinner, useOverlayState } from "@heroui/react";
import type { ComponentType, ReactNode } from "react";

export function PageHeader({
  icon: Icon,
  title,
  description,
  action,
}: {
  icon: ComponentType<{ className?: string }>;
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <header className="relative overflow-hidden rounded-3xl border border-default-200 bg-background p-6 shadow-sm">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_right,hsl(var(--heroui-accent)/0.14),transparent_34rem)]" />
      <div className="relative flex flex-wrap items-start justify-between gap-4">
        <div className="flex items-start gap-4">
          <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-accent/10 text-accent ring-1 ring-accent/15">
            <Icon className="h-5 w-5" />
          </div>
          <div className="min-w-0">
            <h1 className="text-3xl font-semibold tracking-tight">{title}</h1>
            <p className="mt-1 max-w-2xl text-sm text-default-500">
              {description}
            </p>
          </div>
        </div>
        {action}
      </div>
    </header>
  );
}

export function FormModal({
  state,
  title,
  description,
  size = "md",
  children,
}: {
  state: ReturnType<typeof useOverlayState>;
  title: string;
  description?: string;
  size?: "xs" | "sm" | "md" | "lg" | "full" | "cover";
  children: ReactNode;
}) {
  return (
    <Modal state={state}>
      <Modal.Backdrop>
        <Modal.Container size={size} placement="center" scroll="inside">
          <Modal.Dialog>
            <Modal.Header className="flex flex-col gap-1 border-b border-default-100 pb-4">
              <Modal.Heading className="text-lg font-semibold">
                {title}
              </Modal.Heading>
              {description && (
                <p className="text-sm text-default-500">{description}</p>
              )}
            </Modal.Header>
            <Modal.Body className="flex flex-col gap-4 pt-4">{children}</Modal.Body>
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </Modal>
  );
}

export function ModalActions({ children }: { children: ReactNode }) {
  return (
    <div className="flex justify-end gap-2 border-t border-default-100 pt-4">
      {children}
    </div>
  );
}

export function LoadingBlock() {
  return (
    <div className="flex justify-center py-16">
      <Spinner />
    </div>
  );
}

export function EmptyBlock({
  icon: Icon,
  title,
  description,
  action,
}: {
  icon: ComponentType<{ className?: string }>;
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-col items-center gap-3 rounded-3xl border border-dashed border-default-200 bg-background px-6 py-16 text-center shadow-sm">
      <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-default-100 text-default-500">
        <Icon className="h-6 w-6" />
      </div>
      <div>
        <h3 className="text-base font-medium">{title}</h3>
        <p className="mt-1 text-sm text-default-500">{description}</p>
      </div>
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}

export function Alert({ message }: { message: string }) {
  return (
    <div
      role="alert"
      className="rounded-lg border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger"
    >
      {message}
    </div>
  );
}

export function NativeSelect({
  value,
  onChange,
  children,
  ...rest
}: React.SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      value={value}
      onChange={onChange}
      className="h-10 rounded-xl border border-default-200 bg-background px-3 text-sm transition focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/20"
      {...rest}
    >
      {children}
    </select>
  );
}

export function FieldShell({
  label,
  children,
  hint,
}: {
  label: string;
  children: ReactNode;
  hint?: string;
}) {
  return (
    <label className="flex flex-col gap-1.5 text-sm">
      <span className="font-medium text-default-700">{label}</span>
      {children}
      {hint && <span className="text-xs text-default-500">{hint}</span>}
    </label>
  );
}
