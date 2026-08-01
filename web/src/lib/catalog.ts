import type { Package, PackageKind, TrustLevel } from "../types";

export type DiscoveryMode = "pandora" | "agent";
export type SignatureFilter = "all" | "signed" | "unsigned";

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

export const hasSignature = (pkg: Package): boolean =>
  Boolean(pkg.trust.signature && pkg.trust.public_key);

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
  ).sort((left, right) => left.localeCompare(right));

export const getLicenseOptions = (packages: Package[]): string[] =>
  Array.from(new Set(packages.map((pkg) => pkg.license).filter(Boolean))).sort((left, right) =>
    left.localeCompare(right),
  );

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
