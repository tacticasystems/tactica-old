import { createFileRoute, redirect, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import type { FormEvent } from "react";

import { verifyEmail } from "../../auth/api";
import { sessionQueryKey, sessionQueryOptions } from "../../auth/session";

export const Route = createFileRoute("/auth/verify-email")({
  beforeLoad: async ({ context }) => {
    const session = await context.queryClient.ensureQueryData(sessionQueryOptions());
    if (!session) {
      throw redirect({ to: "/auth/login", search: { returnTo: undefined } });
    }
    if (session.email_verified) throw redirect({ to: "/orgs" });
  },
  component: VerifyEmailPage,
});

function VerifyEmailPage() {
  const navigate = useNavigate();
  const { queryClient } = Route.useRouteContext();
  const [error, setError] = useState<string>();
  const [isSubmitting, setIsSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    setIsSubmitting(true);
    try {
      const code = new FormData(event.currentTarget).get("code");
      await verifyEmail(typeof code === "string" ? code : "");
      await queryClient.invalidateQueries({ queryKey: sessionQueryKey });
      await navigate({ to: "/orgs" });
    } catch {
      setError("That verification code is invalid or expired");
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <main>
      <h1>Verify your email</h1>
      <p>Enter the verification code sent to your email address.</p>
      <form onSubmit={handleSubmit}>
        <label>
          Verification code
          <input name="code" required autoComplete="one-time-code" />
        </label>
        {error && <p role="alert">{error}</p>}
        <button type="submit" disabled={isSubmitting}>
          {isSubmitting ? "Verifying…" : "Verify email"}
        </button>
      </form>
    </main>
  );
}
