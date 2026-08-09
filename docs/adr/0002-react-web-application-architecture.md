# Use TanStack Router with runtime-configured API access

The Tactica web application uses Vite, React, and TypeScript with TanStack
Router and TanStack Query. TanStack Router provides typed organisation-slug
routes, validated `returnTo` search parameters, and route-level authentication
guards; TanStack Query is the single cache for the existing Session endpoint.
The browser API client is an application-owned injectable transport, initially
backed by `fetch`, and reads the complete API base URL from a runtime
`/config.js` file so one container image can target different API deployments
without rebuilding.

## Considered options

React Router was considered but rejected because this application benefits from
TanStack Router's stronger type-safety for route parameters and search
parameters, especially for organisation-scoped URLs and safe login return paths.

Build-time Vite environment variables were considered but rejected because
container deployments need to change the API origin at runtime.

## Consequences

The web app must serve runtime configuration before booting and must configure
the API deployment to trust `https://app.tactica.systems`. Cookie-based
authentication requires credentialed requests and CSRF handling; authenticated
route guards redirect unverified Accounts to email verification and expired
Sessions to login.
