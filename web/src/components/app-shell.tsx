import { useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { logout } from "../auth/api";
import { sessionQueryKey } from "../auth/session";

export function AppShell({ children }: { children: React.ReactNode }) {
	const navigate = useNavigate();
	const queryClient = useQueryClient();
	const [isLoggingOut, setIsLoggingOut] = useState(false);

	async function handleLogout() {
		setIsLoggingOut(true);
		try {
			await logout();
		} finally {
			queryClient.removeQueries({ queryKey: sessionQueryKey });
			await navigate({ to: "/auth/login", search: { returnTo: undefined } });
			setIsLoggingOut(false);
		}
	}

	return (
		<>
			<header className="app-header">
				<Link to="/orgs" className="brand">
					Tactica
				</Link>
				<button type="button" onClick={() => void handleLogout()} disabled={isLoggingOut}>
					{isLoggingOut ? "Signing out…" : "Sign out"}
				</button>
			</header>
			{children}
		</>
	);
}
