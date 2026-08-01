import type { PackageListResponse } from "../types";

export interface SearchPackagesInput {
  q: string;
  limit?: number;
  offset?: number;
  signal?: AbortSignal;
}

export interface PalaceClientOptions {
  production?: boolean;
}

export class PalaceApiError extends Error {
  readonly status: number;
  readonly code?: string;

  constructor(message: string, status: number, code?: string) {
    super(message);
    this.name = "PalaceApiError";
    this.status = status;
    this.code = code;
  }
}

const localDevelopmentHosts = new Set(["127.0.0.1", "localhost", "[::1]"]);

const normalizeBaseUrl = (baseUrl: string | undefined, production: boolean): string => {
  if (!baseUrl) {
    return "";
  }

  let url: URL;
  try {
    url = new URL(baseUrl);
  } catch {
    throw new Error("VITE_PALACE_API_URL must be an absolute HTTP or HTTPS URL");
  }

  if (url.protocol === "https:") {
    return baseUrl.replace(/\/+$/, "");
  }

  if (url.protocol !== "http:") {
    throw new Error("VITE_PALACE_API_URL must use HTTPS");
  }

  if (production) {
    throw new Error("VITE_PALACE_API_URL must use HTTPS in production");
  }

  if (!localDevelopmentHosts.has(url.hostname)) {
    throw new Error("HTTP is allowed only for local development");
  }

  return baseUrl.replace(/\/+$/, "");
};

const createPackageDownloadUrl = (baseUrl: string, packageId: string): string => {
  const encodedPackageId = encodeURIComponent(packageId);
  return `${baseUrl}/api/v1/packages/${encodedPackageId}/download`;
};

const createSearchUrl = (baseUrl: string, query: SearchPackagesInput): string => {
  const params = new URLSearchParams({
    q: query.q,
    limit: String(query.limit ?? 24),
    offset: String(query.offset ?? 0),
  });

  if (!baseUrl) {
    return `/api/v1/search?${params.toString()}`;
  }

  return `${baseUrl}/api/v1/search?${params.toString()}`;
};

export const createPalaceClient = (
  baseUrl = import.meta.env.VITE_PALACE_API_URL,
  options: PalaceClientOptions = {},
) => {
  const normalizedBaseUrl = normalizeBaseUrl(
    baseUrl,
    options.production ?? import.meta.env.PROD,
  );

  return {
    getPackageDownloadUrl(packageId: string): string {
      return createPackageDownloadUrl(normalizedBaseUrl, packageId);
    },

    async searchPackages(input: SearchPackagesInput): Promise<PackageListResponse> {
      const response = await fetch(createSearchUrl(normalizedBaseUrl, input), {
        headers: {
          Accept: "application/json",
        },
        signal: input.signal,
      });

      if (!response.ok) {
        let message = `Search failed with status ${response.status}`;
        let code: string | undefined;

        try {
          const payload = (await response.json()) as {
            code?: string;
            message?: string;
          };
          message = payload.message ?? message;
          code = payload.code;
        } catch {
          message = response.statusText || message;
        }

        throw new PalaceApiError(message, response.status, code);
      }

      return (await response.json()) as PackageListResponse;
    },
  };
};
