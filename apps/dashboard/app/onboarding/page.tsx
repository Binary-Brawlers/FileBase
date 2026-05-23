import { Onboarding } from "./Onboarding";
import { Brand } from "../../components/Brand";
import { ThemeToggle } from "../../components/ThemeToggle";

export const dynamic = "force-dynamic";

export default function OnboardingPage() {
  return (
    <div className="auth-shell min-h-screen flex flex-col">
      <header className="flex items-center justify-between p-6 sm:px-10">
        <Brand tagline="Setup wizard" />
        <ThemeToggle />
      </header>
      <main className="flex-1 flex items-start justify-center px-4 pb-12 sm:px-6">
        <Onboarding />
      </main>
    </div>
  );
}
