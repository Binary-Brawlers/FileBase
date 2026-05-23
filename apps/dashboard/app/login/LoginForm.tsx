"use client";

import { Button, Card, FieldError, Input, Label, TextField } from "@heroui/react";
import { useRouter } from "next/navigation";
import { useState, type FormEvent } from "react";
import { ApiError } from "../../lib/api";
import { setToken } from "../../lib/auth";
import { useLogin } from "../../lib/queries";
import { ThemeToggle } from "../../components/ThemeToggle";

export function LoginForm() {
  const router = useRouter();
  const login = useLogin();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      const res = await login.mutateAsync({ email, password });
      setToken(res.token);
      router.push("/dashboard");
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Login failed.");
    }
  }

  return (
    <Card className="w-full shadow-xl border border-default-200/60">
      <Card.Header className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <Card.Title className="text-2xl">Welcome back</Card.Title>
          <Card.Description>
            Sign in to your FileBase admin account.
          </Card.Description>
        </div>
        <div className="hidden lg:block">
          <ThemeToggle />
        </div>
      </Card.Header>
      <Card.Content>
        <form onSubmit={onSubmit} className="flex flex-col gap-5">
          <TextField type="email" isRequired>
            <Label>Email</Label>
            <Input
              autoComplete="email"
              placeholder="you@company.com"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
            />
            <FieldError />
          </TextField>
          <TextField type="password" isRequired>
            <Label>Password</Label>
            <Input
              autoComplete="current-password"
              placeholder="••••••••"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
            <FieldError />
          </TextField>
          {error && (
            <div
              role="alert"
              className="rounded-lg border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger"
            >
              {error}
            </div>
          )}
          <Button
            type="submit"
            variant="primary"
            size="lg"
            fullWidth
            isPending={login.isPending}
            isDisabled={!email || !password}
          >
            Sign in
          </Button>
        </form>
      </Card.Content>
      <Card.Footer>
        <p className="text-xs text-default-500">
          Need to set up FileBase?{" "}
          <a href="/onboarding" className="text-accent font-medium hover:underline">
            Run onboarding
          </a>
        </p>
      </Card.Footer>
    </Card>
  );
}
