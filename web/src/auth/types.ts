export interface Session {
  account_id: string;
  email_verified: boolean;
  session: {
    created_at: string;
    idle_expires_at: string;
    absolute_expires_at: string;
  };
}

export interface Credentials {
  email: string;
  password: string;
}
