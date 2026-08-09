import { ApiError, apiClient } from "../api/client";
import type { Credentials, Session } from "./types";

export function isUnauthorized(error: unknown): boolean {
  return error instanceof ApiError && error.status === 401;
}

export async function getSession(): Promise<Session | null> {
  try {
    return await apiClient.request<Session>("/auth/session");
  } catch (error) {
    if (isUnauthorized(error)) return null;
    throw error;
  }
}

export function login(credentials: Credentials) {
  return apiClient.request<{ account_id: string; email_verified: boolean }>(
    "/auth/login",
    { method: "POST", body: JSON.stringify(credentials) },
  );
}

export function verifyEmail(code: string) {
  return apiClient.request<void>("/auth/verify-email", {
    method: "POST",
    body: JSON.stringify({ code }),
  });
}

export function logout() {
  return apiClient.request<void>("/auth/logout", { method: "POST" });
}
