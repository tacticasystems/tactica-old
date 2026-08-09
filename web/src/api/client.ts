import { config } from "../config";

export type Transport = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;

export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly code?: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

function readCookie(name: string): string | undefined {
  return document.cookie
    .split(";")
    .map((part) => part.trim().split("="))
    .find(([key]) => key === name)?.[1];
}

export class ApiClient {
  constructor(
    private readonly baseUrl: string,
    private readonly transport: Transport = (input, init) => fetch(input, init),
  ) {}

  async request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const method = init.method?.toUpperCase() ?? "GET";
    const headers = new Headers(init.headers);

    if (init.body && !headers.has("content-type")) {
      headers.set("content-type", "application/json");
    }
    if (!["GET", "HEAD", "OPTIONS"].includes(method)) {
      const csrfToken = readCookie("tactica_csrf");
      if (csrfToken) headers.set("x-csrf-token", csrfToken);
    }

    const response = await this.transport(`${this.baseUrl}${path}`, {
      ...init,
      headers,
      credentials: "include",
    });

    if (!response.ok) {
      let message = response.statusText || "The API request failed";
      let code: string | undefined;
      try {
        const error = (await response.json()) as {
          message?: string;
          code?: string;
        };
        message = error.message ?? message;
        code = error.code;
      } catch {
        // Preserve the HTTP status when the response is not JSON.
      }
      throw new ApiError(message, response.status, code);
    }

    if (response.status === 204) return undefined as T;
    return (await response.json()) as T;
  }
}

export const apiClient = new ApiClient(config.apiBaseUrl);
