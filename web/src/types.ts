export type TrustLevel =
  | "experimental"
  | "community"
  | "verified"
  | "official"
  | "enterprise"
  | "certified";

export type PackageKind =
  | "gene"
  | "domain_harness"
  | "meta_harness"
  | "source_harness"
  | "package"
  | "provider"
  | "skill"
  | "memory_schema"
  | "runtime_extension"
  | "capability_pack"
  | "template"
  | "persona"
  | "policy"
  | "benchmark"
  | "dataset"
  | "plugin"
  | "connector"
  | "sdk"
  | "distribution";

export interface TrustInfo {
  level: TrustLevel;
  signature: string | null;
  public_key: string | null;
  content_hash: string | null;
  publisher: string;
}

export interface CapabilityInfo {
  provides: string[];
  requires: string[];
}

export interface CompatibilityInfo {
  runtimes: string[];
  platforms: string[];
}

export interface Provenance {
  source?: string;
  commit?: string;
  tag?: string;
  release_id?: string;
}

export interface Package {
  id: string;
  name: string;
  version: string;
  kind: PackageKind;
  description: string;
  author: string;
  license: string;
  trust: TrustInfo;
  capabilities: CapabilityInfo;
  downloads: number;
  success_rate: number;
  compatibility: CompatibilityInfo;
  repository: string | null;
  artifact_url: string | null;
  homepage: string | null;
  tags: string[];
  yanked: boolean;
  deprecated: string | null;
  provenance: Provenance | null;
  created_at: string;
  updated_at: string;
}

export interface PackageListResponse {
  total: number;
  limit: number;
  offset: number;
  packages: Package[];
}
