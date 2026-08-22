import { redirect } from "next/navigation";
import { Brand } from "../../components/Brand";
import { ThemeToggle } from "../../components/ThemeToggle";
import { api } from "../../lib/api";
import { SignupForm } from "./SignupForm";

export const dynamic = "force-dynamic";

export default async function SignupPage() {
  const status = await api.getSetupStatus().catch(() => null);
  if (!status?.registration_enabled) {
    redirect("/login");
  }

  return (
    <div className="auth-shell min-h-screen grid lg:grid-cols-[1.1fr_1fr]">
      <aside className="brand-panel dot-grid hidden lg:flex flex-col justify-between p-12 border-r border-default-200">
        <Brand size="lg" tagline="Managed file uploads" />
        <div className="flex max-w-md flex-col gap-4">
          <h2 className="text-3xl font-semibold tracking-tight">
            Your upload infrastructure, ready in minutes.
          </h2>
          <p className="text-default-600">
            Create a workspace, connect your storage, and issue scoped API keys
            without operating the FileBase gateway yourself.
          </p>
          <ul className="flex flex-col gap-2 text-sm text-default-600">
            <li className="flex items-center gap-2">
              <span className="size-1.5 rounded-full bg-accent" />
              Isolated project and team access
            </li>
            <li className="flex items-center gap-2">
              <span className="size-1.5 rounded-full bg-accent" />
              Bring your own FTP, SFTP, or object storage
            </li>
            <li className="flex items-center gap-2">
              <span className="size-1.5 rounded-full bg-accent" />
              Signed uploads and media processing
            </li>
          </ul>
        </div>
        <p className="text-xs text-default-500">
          © {new Date().getFullYear()} FileBase
        </p>
      </aside>

      <main className="flex flex-col items-center justify-center p-6 sm:p-10">
        <div className="flex w-full max-w-md flex-col gap-6">
          <div className="flex items-center justify-between lg:hidden">
            <Brand />
            <ThemeToggle />
          </div>
          <SignupForm />
        </div>
      </main>
    </div>
  );
}
