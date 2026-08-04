create table account
(
  account_id  uuid  primary key  default uuidv7(),
  email       text  not null     unique
);

create table identity
(
  identity_id  uuid   primary key  default uuidv7(),
  account_id   uuid   not null     references account(account_id) on delete cascade,
  data         jsonb  not null
);
