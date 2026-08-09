import { Link } from "@tanstack/react-router";

export function NotFound() {
  return (
    <main>
      <h1>Organisation not found</h1>
      <p>That organisation does not exist or you do not have access to it.</p>
      <Link to="/orgs">Back to organisations</Link>
    </main>
  );
}
