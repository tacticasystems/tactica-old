import { createFileRoute, redirect, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import type { FormEvent } from "react";

import { isUnauthorized, login } from "../../auth/api";
import { sessionQueryKey, sessionQueryOptions } from "../../auth/session";
import type { Credentials } from "../../auth/types";

function safeReturnTo(value: unknown): string | undefined {
	return typeof value === "string" && value.startsWith("/") && !value.startsWith("//")
		? value
		: undefined;
}

export const Route = createFileRoute("/auth/login")({
	validateSearch: (search) => ({ returnTo: safeReturnTo(search.returnTo) }),
	beforeLoad: async ({ context }) => {
		const session = await context.queryClient.ensureQueryData(sessionQueryOptions());
		if (session) throw redirect({ to: "/orgs" });
	},
	component: LoginPage,
});

function LoginPage() {
	const navigate = useNavigate();
	const { queryClient } = Route.useRouteContext();
	const { returnTo } = Route.useSearch();
	const [credentials, setCredentials] = useState<Credentials>({ email: "", password: "" });
	const [error, setError] = useState<string>();
	const [isSubmitting, setIsSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    setIsSubmitting(true);
    try {
      await login(credentials);
      await queryClient.refetchQueries({ queryKey: sessionQueryKey, type: "all" });
      await navigate({ to: returnTo ?? "/orgs" });
    } catch (cause) {
      setError(isUnauthorized(cause) ? "Invalid email or password" : "Unable to sign in right now");
    } finally {
      setIsSubmitting(false);
    }
  }

	return (
		<main>
			<h1>Sign in to Tactica</h1>
			<form onSubmit={handleSubmit}>
				<label>
					Email
					<input
						type="email"
						autoComplete="email"
						required
						value={credentials.email}
						onChange={(event) => setCredentials({ ...credentials, email: event.target.value })}
					/>
				</label>
				<label>
					Password
					<input
						type="password"
						autoComplete="current-password"
						required
						value={credentials.password}
						onChange={(event) => setCredentials({ ...credentials, password: event.target.value })}
					/>
				</label>
				{error && <p role="alert">{error}</p>}
				<button type="submit" disabled={isSubmitting}>
					{isSubmitting ? "Signing in…" : "Sign in"}
				</button>
			</form>
		</main>
	);
}
