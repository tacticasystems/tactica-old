import { describe, expect, it, vi } from "vitest";

import { ApiClient } from "./client";

describe("ApiClient", () => {
	it("uses cookie credentials and forwards the CSRF cookie for unsafe requests", async () => {
		vi.spyOn(document, "cookie", "get").mockReturnValue("tactica_csrf=test-csrf-token");
		const transport = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
			void input;
			void init;
			return new Response(JSON.stringify({ ok: true }), {
				status: 200,
				headers: { "content-type": "application/json" },
			});
		});

		await new ApiClient("https://api.example.test/api/v1", transport).request("/auth/logout", {
			method: "POST",
		});

		expect(transport.mock.calls[0]?.[0]).toBe("https://api.example.test/api/v1/auth/logout");
		const request = transport.mock.calls[0]?.[1];
		expect(request?.credentials).toBe("include");
		expect(new Headers(request?.headers).get("x-csrf-token")).toBe("test-csrf-token");
	});
});
