import { redirect } from "next/navigation";
import { api } from "../lib/api";

export const dynamic = "force-dynamic";

export default async function HomePage() {
  try {
    const status = await api.getSetupStatus();
    redirect(status.setup_required ? "/onboarding" : "/login");
  } catch {
    redirect("/onboarding");
  }
}
