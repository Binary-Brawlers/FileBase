"use client";

import {
  Button,
  Card,
  FieldError,
  Input,
  Label,
  TextField,
} from "@heroui/react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState, type FormEvent } from "react";
import { ThemeToggle } from "../../components/ThemeToggle";
import { ApiError } from "../../lib/api";
import { setToken } from "../../lib/auth";
import { useRegister } from "../../lib/queries";

export function SignupForm() {
  const router = useRouter();
  const register = useRegister();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [projectName, setProjectName] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    try {
      const session = await register.mutateAsync({
        name,
        email,
        password,
        project_name: projectName || undefined,
      });
      setToken(session.token);
      router.push("/dashboard/storage");
    } catch (err) {
      setError(
        err instanceof ApiError ? err.message : "Account creation failed.",
      );
    }
  }

  return (
    <Card className="w-full border border-default-200/60 shadow-xl">
      <Card.Header className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <Card.Title className="text-2xl">Create your account</Card.Title>
          <Card.Description>
            We’ll create your first project automatically.
          </Card.Description>
        </div>
        <div className="hidden lg:block">
          <ThemeToggle />
        </div>
      </Card.Header>
      <Card.Content>
        <form onSubmit={onSubmit} className="flex flex-col gap-5">
          <TextField isRequired>
            <Label>Name</Label>
            <Input
              autoComplete="name"
              placeholder="Ada Lovelace"
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
            <FieldError />
          </TextField>
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
              autoComplete="new-password"
              placeholder="At least 8 characters"
              minLength={8}
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
            <FieldError />
          </TextField>
          <TextField>
            <Label>First project</Label>
            <Input
              placeholder="My Project"
              value={projectName}
              onChange={(event) => setProjectName(event.target.value)}
            />
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
            isPending={register.isPending}
            isDisabled={!name || !email || password.length < 8}
          >
            Create account
          </Button>
        </form>
      </Card.Content>
      <Card.Footer>
        <p className="text-sm text-default-500">
          Already have an account?{" "}
          <Link
            href="/login"
            className="font-medium text-accent hover:underline"
          >
            Sign in
          </Link>
        </p>
      </Card.Footer>
    </Card>
  );
}
