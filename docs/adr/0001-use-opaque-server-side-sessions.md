# Use opaque server-side sessions

Tactica uses high-entropy opaque session credentials and stores only their
SHA-256 hashes in PostgreSQL, rather than putting authentication state in
self-contained tokens. This adds a database lookup and occasional activity
write to authenticated requests, but gives us immediate revocation, bounded
sliding expiry, server-side CSRF association, and one validation model for
cookie and bearer clients.
