create table account
(
  account_id        uuid         primary key default uuidv7(),
  email             text         not null unique,
  email_verified_at timestamptz,
  constraint account_email_canonical check (email = lower(btrim(email)))
);

create table identity
(
  identity_id  uuid  primary key default uuidv7(),
  account_id   uuid  not null references account(account_id) on delete cascade,
  kind         text  not null check (kind in ('password', 'external')),
  password_hash text,
  provider     text,
  subject      text,
  constraint identity_shape check (
    (kind = 'password' and password_hash is not null and provider is null and subject is null)
    or
    (kind = 'external' and password_hash is null and provider is not null and subject is not null)
  )
);

create unique index identity_one_password_per_account
  on identity (account_id) where kind = 'password';
create unique index identity_external_subject_unique
  on identity (provider, subject) where kind = 'external';

create table session
(
  session_id          uuid        primary key default uuidv7(),
  account_id          uuid        not null references account(account_id) on delete cascade,
  token_hash          bytea       not null unique,
  csrf_token_hash     bytea       not null,
  created_at          timestamptz not null,
  idle_expires_at     timestamptz not null,
  absolute_expires_at timestamptz not null,
  revoked_at          timestamptz,
  constraint session_expiry_order check (
    created_at < idle_expires_at and idle_expires_at <= absolute_expires_at
  )
);

create index session_account_id_idx on session (account_id);

create table email_verification_challenge
(
  challenge_id uuid        primary key default uuidv7(),
  account_id   uuid        not null references account(account_id) on delete cascade,
  code_hash    bytea       not null unique,
  expires_at   timestamptz not null,
  failed_attempts smallint not null default 0 check (failed_attempts between 0 and 5),
  consumed_at  timestamptz
);

create unique index email_verification_one_active_per_account
  on email_verification_challenge (account_id) where consumed_at is null;
