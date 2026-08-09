import { createFileRoute, redirect } from "@tanstack/react-router";

import { sessionQueryOptions } from "../auth/session";

export const Route = createFileRoute("/orgs")({
  beforeLoad: async ({ context, location }) => {
    const session = await context.queryClient.ensureQueryData(sessionQueryOptions());
    if (!session) {
      throw redirect({ to: "/auth/login", search: { returnTo: location.href } });
    }
    if (!session.email_verified) throw redirect({ to: "/auth/verify-email" });
  },
  component: () => (
    <main>
      <h1>Your organisations</h1>
      <p>No organisations are available yet.</p>
      <button type="button" disabled>
        Create organisation
      </button>
    </main>
  ),
});
