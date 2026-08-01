import type { Package, PackageKind, TrustLevel } from "../types";

export type DiscoveryMode = "pandora" | "agent";
export type SignatureStatus = "verified" | "not_verified" | "metadata_only" | "unreported";
export type SignatureFilter = "all" | SignatureStatus;

const MAX_METADATA_ITEMS = 12;
const MAX_METADATA_TEXT_LENGTH = 160;
const MAX_FILTER_OPTIONS = 64;

export const trustLevels: TrustLevel[] = [
  "experimental",
  "community",
  "verified",
  "official",
  "enterprise",
  "certified",
];

export const pandoraKinds = new Set<PackageKind>([
  "gene",
  "domain_harness",
  "meta_harness",
  "source_harness",
  "skill",
  "connector",
  "provider",
  "plugin",
]);

export const agentKinds = new Set<PackageKind>([
  "connector",
  "provider",
  "runtime_extension",
  "plugin",
  "sdk",
  "distribution",
]);

const agentRuntimeTokens = [
  "codex",
  "claude",
  "aider",
  "cline",
  "continue",
  "goose",
  "opencode",
];

export const formatLabel = (value: string): string =>
  value
    .replace(/_/g, " ")
    .replace(/\b\w/g, (character) => character.toUpperCase());

export const truncateText = (value: unknown, maxLength = MAX_METADATA_TEXT_LENGTH): string => {
  if (typeof value !== "string") {
    return "";
  }

  return value.length > maxLength ? `${value.slice(0, Math.max(1, maxLength - 1))}…` : value;
};

export interface BoundedMetadata {
  items: string[];
  remaining: number;
}

export const boundMetadata = (values: unknown, limit = MAX_METADATA_ITEMS): BoundedMetadata => {
  if (!Array.isArray(values) || limit < 1) {
    return { items: [], remaining: 0 };
  }

  const items = values
    .slice(0, limit)
    .map((value) => truncateText(value))
    .filter((value) => value.length > 0);

  return {
    items,
    remaining: Math.max(0, values.length - limit),
  };
};

export const hasSignatureMetadata = (pkg: Package): boolean =>
  Boolean(pkg.trust.signature && pkg.trust.public_key);

export const getSignatureStatus = (pkg: Package): SignatureStatus => {
  if (pkg.trust.signature_verified === true) {
    return "verified";
  }

  if (pkg.trust.signature_verified === false) {
    return "not_verified";
  }

  return hasSignatureMetadata(pkg) ? "metadata_only" : "unreported";
};

export const getSignatureLabel = (pkg: Package): string => {
  switch (getSignatureStatus(pkg)) {
    case "verified":
      return "Verified by Palace";
    case "not_verified":
      return "Not verified";
    case "metadata_only":
      return "Metadata available";
    case "unreported":
      return "Verification not reported";
  }
};

export const getSignatureDescription = (pkg: Package): string => {
  switch (getSignatureStatus(pkg)) {
    case "verified":
      return "The registry explicitly reports this version as verified.";
    case "not_verified":
      return "The registry explicitly reports that this version is not verified.";
    case "metadata_only":
      return "Signature and public-key metadata are available; the registry does not report verification.";
    case "unreported":
      return "The registry does not report a signature verification state for this version.";
  }
};

export const matchesMode = (pkg: Package, mode: DiscoveryMode): boolean => {
  const runtimeValues = pkg.compatibility.runtimes.map((runtime) => runtime.toLowerCase());
  const supportsPandora = runtimeValues.some((runtime) => runtime.includes("pandora"));
  const supportsExternalAgent = runtimeValues.some((runtime) =>
    agentRuntimeTokens.some((token) => runtime.includes(token)),
  );

  if (mode === "pandora") {
    return supportsPandora || pandoraKinds.has(pkg.kind);
  }

  return supportsExternalAgent || agentKinds.has(pkg.kind);
};

export const getCompatibilityOptions = (packages: Package[]): string[] =>
  Array.from(
    new Set(
      packages.flatMap((pkg) => pkg.compatibility.runtimes).filter((runtime) => runtime.trim().length > 0),
    ),
  )
    .sort((left, right) => left.localeCompare(right))
    .slice(0, MAX_FILTER_OPTIONS);

export const getLicenseOptions = (packages: Package[]): string[] =>
  Array.from(new Set(packages.map((pkg) => pkg.license).filter(Boolean)))
    .sort((left, right) => left.localeCompare(right))
    .slice(0, MAX_FILTER_OPTIONS);

export const formatSuccessRate = (value: number): string => `${Math.round(value * 100)}%`;

export const formatDownloads = (value: number): string =>
  new Intl.NumberFormat("en-US", { notation: "compact", maximumFractionDigits: 1 }).format(value);

export const relativeFreshness = (isoDate: string): string => {
  const timestamp = new Date(isoDate).getTime();

  if (Number.isNaN(timestamp)) {
    return "Date unavailable";
  }

  const diffDays = Math.max(0, Math.floor((Date.now() - timestamp) / 86_400_000));

  if (diffDays === 0) {
    return "Updated today";
  }

  if (diffDays === 1) {
    return "Updated 1 day ago";
  }

  return `Updated ${diffDays} days ago`;
};
