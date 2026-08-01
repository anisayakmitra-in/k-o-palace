import type { PackageListResponse } from "../types";

export interface SearchPackagesInput {
  q: string;
  limit?: number;
  offset?: number;
  signal?: AbortSignal;
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

const normalizeBaseUrl = (baseUrl?: string): string =>
  baseUrl ? baseUrl.replace(/\/+$/, "") : "";

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

export const createPalaceClient = (baseUrl = import.meta.env.VITE_PALACE_API_URL) => {
  const normalizedBaseUrl = normalizeBaseUrl(baseUrl);

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
