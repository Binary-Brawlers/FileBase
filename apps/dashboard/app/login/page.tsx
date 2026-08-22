import { LoginForm } from "./LoginForm";
import { Brand } from "../../components/Brand";
import { ThemeToggle } from "../../components/ThemeToggle";

export const dynamic = "force-dynamic";

export default function LoginPage() {
  return (
    <div className="auth-shell min-h-screen grid lg:grid-cols-[1.1fr_1fr]">
      <aside className="brand-panel dot-grid hidden lg:flex flex-col justify-between p-12 border-r border-default-200">
        <Brand size="lg" tagline="Self-hosted file uploads" />
        <div className="flex flex-col gap-4 max-w-md">
          <h2 className="text-3xl font-semibold tracking-tight">
            One dashboard for every upload.
          </h2>
          <p className="text-default-600">
            Manage storage connections, presets, and API keys for your apps —
            all from a single place that you control.
          </p>
          <ul className="flex flex-col gap-2 text-sm text-default-600">
            <li className="flex items-center gap-2">
              <span className="size-1.5 rounded-full bg-accent" />
              Local, FTP, and SFTP storage backends
            </li>
            <li className="flex items-center gap-2">
              <span className="size-1.5 rounded-full bg-accent" />
              Reusable upload presets with duplicate handling
            </li>
            <li className="flex items-center gap-2">
              <span className="size-1.5 rounded-full bg-accent" />
              Live and test API keys per project
            </li>
          </ul>
        </div>
        <p className="text-xs text-default-500">
          © {new Date().getFullYear()} FileBase
        </p>
      </aside>

      <main className="flex flex-col items-center justify-center p-6 sm:p-10">
        <div className="w-full max-w-md flex flex-col gap-6">
          <div className="flex items-center justify-between lg:hidden">
            <Brand />
            <ThemeToggle />
          </div>
          <LoginForm />
        </div>
      </main>
    </div>
  );
}
