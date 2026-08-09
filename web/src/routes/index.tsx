import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
  component: () => (
    <main>
      <h1>Tactica</h1>
      <p>The application shell is loading.</p>
    </main>
  ),
});
