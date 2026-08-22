import { AcceptInviteForm } from "./AcceptInviteForm";
import { Brand } from "../../components/Brand";
import { ThemeToggle } from "../../components/ThemeToggle";

export const dynamic = "force-dynamic";

export default function AcceptInvitePage() {
  return (
    <main className="auth-shell min-h-screen px-6 py-10 sm:px-10">
      <div className="mx-auto flex w-full max-w-xl flex-col gap-10">
        <header className="flex items-center justify-between">
          <Brand tagline="Team invitation" />
          <ThemeToggle />
        </header>
        <AcceptInviteForm />
      </div>
    </main>
  );
}
