export interface TacticaConfig {
  apiBaseUrl: string;
}

declare global {
  interface Window {
    __TACTICA_CONFIG__?: Partial<TacticaConfig>;
  }
}

function readConfig(): TacticaConfig {
  const apiBaseUrl = window.__TACTICA_CONFIG__?.apiBaseUrl;

  if (!apiBaseUrl) {
    throw new Error("Tactica runtime configuration is missing apiBaseUrl");
  }

  try {
    const url = new URL(apiBaseUrl);
    if (!url.protocol.startsWith("http")) {
      throw new Error("the API base URL must use HTTP or HTTPS");
    }
  } catch (error) {
    throw new Error(
      `Tactica runtime configuration has an invalid apiBaseUrl: ${String(error)}`,
    );
  }

  return { apiBaseUrl: apiBaseUrl.replace(/\/$/, "") };
}

export const config = readConfig();
