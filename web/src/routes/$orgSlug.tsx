import { createFileRoute, redirect } from "@tanstack/react-router";

import { sessionQueryOptions } from "../auth/session";
import { AppShell } from "../components/app-shell";
import { NotFound } from "../components/not-found";

export const Route = createFileRoute("/$orgSlug")({
	beforeLoad: async ({ context, location }) => {
		const session = await context.queryClient.ensureQueryData(sessionQueryOptions());
		if (!session) {
			throw redirect({ to: "/auth/login", search: { returnTo: location.href } });
		}
		if (!session.email_verified) throw redirect({ to: "/auth/verify-email" });
	},
	component: OrganisationRoute,
});

function OrganisationRoute() {
	return (
		<AppShell>
			<NotFound />
		</AppShell>
	);
}
