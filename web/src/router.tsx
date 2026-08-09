import { createRouter } from "@tanstack/react-router";

import { routeTree } from "./routeTree.gen";

export const router = createRouter({
  routeTree,
  defaultPreload: "intent",
  defaultPendingComponent: () => <p>Loading…</p>,
  defaultErrorComponent: ({ error }) => (
    <main>
      <h1>Something went wrong</h1>
      <p>{error.message}</p>
    </main>
  ),
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
