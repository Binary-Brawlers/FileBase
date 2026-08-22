"use client";

import { Button, Input, Label, Spinner, TextField } from "@heroui/react";
import { ArrowRight, ShieldCheck, UserRoundPlus } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState, type FormEvent } from "react";
import { Alert } from "../../components/PageUI";
import { ApiError } from "../../lib/api";
import { setToken } from "../../lib/auth";
import { useAcceptInvitation, useInvitationPreview } from "../../lib/queries";

export function AcceptInviteForm() {
  const router = useRouter();
  const [token, setInvitationToken] = useState<string | null>(null);
  const preview = useInvitationPreview(token || undefined);
  const accept = useAcceptInvitation();
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const hash = new URLSearchParams(window.location.hash.slice(1));
    setInvitationToken(hash.get("token") ?? "");
  }, []);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!token) return;
    setError(null);
    try {
      const result = await accept.mutateAsync({
        token,
        name: preview.data?.existing_account ? undefined : name,
        password,
      });
      setToken(result.token);
      router.replace("/dashboard");
    } catch (cause) {
      setError(
        cause instanceof ApiError
          ? cause.message
          : "The invitation could not be accepted.",
      );
    }
  }

  if (token === null) {
    return (
      <div className="flex items-center justify-center gap-3 py-16 text-sm text-default-500">
        <Spinner /> Checking invitation…
      </div>
    );
  }

  if (!token) {
    return <InvalidInvitation message="This invitation link is incomplete." />;
  }

  if (preview.isPending) {
    return (
      <div className="flex items-center justify-center gap-3 py-16 text-sm text-default-500">
        <Spinner /> Checking invitation…
      </div>
    );
  }

  if (preview.isError || !preview.data) {
    return (
      <InvalidInvitation message="This invitation is invalid, expired, or has already been used." />
    );
  }

  const invitation = preview.data;
  const canSubmit =
    password.length >= 8 &&
    (invitation.existing_account || name.trim().length > 0);

  return (
    <section className="overflow-hidden rounded-3xl border border-default-200 bg-background shadow-xl">
      <div className="border-b border-default-100 px-6 py-7 sm:px-8">
        <div className="mb-5 flex h-12 w-12 items-center justify-center rounded-2xl bg-accent/10 text-accent">
          <UserRoundPlus className="h-5 w-5" />
        </div>
        <h1 className="text-2xl font-semibold tracking-tight">
          Join {invitation.project_name}
        </h1>
        <p className="mt-2 text-sm text-default-500">
          You were invited as <strong>{invitation.role}</strong> using{" "}
          {invitation.email}.
        </p>
      </div>

      <form onSubmit={submit} className="flex flex-col gap-5 px-6 py-7 sm:px-8">
        <div className="flex items-start gap-3 border-b border-default-100 pb-5 text-sm">
          <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-accent" />
          <div>
            <p className="font-medium capitalize">{invitation.role} access</p>
            <p className="mt-1 text-default-500">
              {roleDescription(invitation.role)}
            </p>
          </div>
        </div>

        {!invitation.existing_account && (
          <TextField isRequired>
            <Label>Your name</Label>
            <Input
              autoComplete="name"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="Ada Lovelace"
            />
          </TextField>
        )}

        <TextField type="password" isRequired>
          <Label>
            {invitation.existing_account
              ? "Confirm your password"
              : "Create a password"}
          </Label>
          <Input
            autoComplete={
              invitation.existing_account ? "current-password" : "new-password"
            }
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            placeholder="At least 8 characters"
          />
        </TextField>

        {invitation.existing_account && (
          <p className="text-xs text-default-500">
            This email already has a FileBase account. Enter its password to
            confirm the invitation belongs to you.
          </p>
        )}
        {error && <Alert message={error} />}

        <Button
          type="submit"
          variant="primary"
          size="lg"
          fullWidth
          isPending={accept.isPending}
          isDisabled={!canSubmit}
        >
          Accept invitation <ArrowRight className="h-4 w-4" />
        </Button>
      </form>
    </section>
  );
}

function InvalidInvitation({ message }: { message: string }) {
  return (
    <section className="rounded-3xl border border-default-200 bg-background px-6 py-12 text-center shadow-xl">
      <h1 className="text-xl font-semibold">Invitation unavailable</h1>
      <p className="mx-auto mt-2 max-w-sm text-sm text-default-500">
        {message}
      </p>
      <Link href="/login" className="mt-6 inline-block">
        <Button variant="secondary">Go to sign in</Button>
      </Link>
    </section>
  );
}

function roleDescription(role: "admin" | "editor" | "viewer") {
  if (role === "admin") {
    return "Manage project settings, storage, API keys, and teammates.";
  }
  if (role === "editor") {
    return "Manage files, upload presets, and webhooks.";
  }
  return "View files, logs, analytics, and project configuration.";
}
