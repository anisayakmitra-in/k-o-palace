//! In-memory package store (will be replaced with PostgreSQL in production).

use crate::types::*;
use chrono::Utc;
use std::collections::HashMap;

pub struct PackageStore {
    packages: HashMap<String, Package>,
}

impl PackageStore {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }

    pub fn list(&self, params: &ListParams) -> Vec<Package> {
        let mut pkgs: Vec<Package> = self.packages.values().cloned().collect();

        // Filter by kind
        if let Some(kind) = &params.kind {
            pkgs.retain(|p| p.kind.as_str() == kind);
        }

        // Filter by tag/category
        if let Some(cat) = &params.category {
            pkgs.retain(|p| p.tags.iter().any(|t| t == cat));
        }

        // Filter by query
        if let Some(q) = &params.q {
            let q = q.to_lowercase();
            pkgs.retain(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
                    || p.id.to_lowercase().contains(&q)
                    || p.tags.iter().any(|t| t.to_lowercase().contains(&q))
            });
        }

        // Sort by downloads (most popular first)
        pkgs.sort_by_key(|p| std::cmp::Reverse(p.downloads));

        // Paginate
        let offset = params.offset.unwrap_or(0);
        let limit = params.limit.unwrap_or(50);
        pkgs.into_iter().skip(offset).take(limit).collect()
    }

    pub fn get(&self, id: &str) -> Option<Package> {
        self.packages.get(id).cloned()
    }

    pub fn search(&self, query: &str) -> Vec<Package> {
        let q = query.to_lowercase();
        let mut results: Vec<Package> = self
            .packages
            .values()
            .filter(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
                    || p.id.to_lowercase().contains(&q)
                    || p.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .cloned()
            .collect();
        results.sort_by_key(|p| std::cmp::Reverse(p.downloads));
        results
    }

    pub fn featured(&self) -> Vec<Package> {
        let mut featured: Vec<Package> = self
            .packages
            .values()
            .filter(|p| {
                matches!(
                    p.trust.level,
                    TrustLevel::Official | TrustLevel::Verified | TrustLevel::Certified
                )
            })
            .cloned()
            .collect();
        featured.sort_by_key(|p| std::cmp::Reverse(p.downloads));
        featured.into_iter().take(10).collect()
    }

    pub fn trending(&self) -> Vec<Package> {
        let mut trending: Vec<Package> = self.packages.values().cloned().collect();
        trending.sort_by_key(|p| std::cmp::Reverse(p.downloads));
        trending.into_iter().take(20).collect()
    }

    pub fn newest(&self) -> Vec<Package> {
        let mut newest: Vec<Package> = self.packages.values().cloned().collect();
        newest.sort_by_key(|p| std::cmp::Reverse(p.created_at));
        newest.into_iter().take(20).collect()
    }

    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self
            .packages
            .values()
            .flat_map(|p| p.tags.clone())
            .collect();
        cats.sort();
        cats.dedup();
        cats
    }

    pub fn publish(&mut self, pkg: Package) -> Result<Package, String> {
        if self.packages.contains_key(&pkg.id) {
            return Err(format!("Package '{}' already exists", pkg.id));
        }
        self.packages.insert(pkg.id.clone(), pkg.clone());
        Ok(pkg)
    }

    /// Seed with sample packages for development.
    pub fn seed_samples(&mut self) {
        let now = Utc::now();

        let samples = vec![
            Package {
                id: "browser.chrome".into(),
                name: "Chrome Browser Gene".into(),
                version: "1.4.0".into(),
                kind: PackageKind::Gene,
                description: "Browser automation gene using Chrome DevTools Protocol".into(),
                author: "openpandora".into(),
                license: "MIT".into(),
                trust: TrustInfo {
                    level: TrustLevel::Official,
                    signature: None,
                    content_hash: None,
                    publisher: "openpandora".into(),
                },
                capabilities: CapabilityInfo {
                    provides: vec!["browser.open".into(), "browser.click".into(), "browser.extract".into()],
                    requires: vec![],
                },
                downloads: 15420,
                success_rate: 0.97,
                compatibility: CompatibilityInfo {
                    runtimes: vec!["pandora>=0.2".into()],
                    platforms: vec!["linux".into(), "macos".into(), "windows".into()],
                },
                repository: Some("https://github.com/openpandora/browser-gene".into()),
                homepage: None,
                tags: vec!["browser".into(), "automation".into(), "multimodal".into()],
                created_at: now,
                updated_at: now,
            },
            Package {
                id: "filesystem.gene".into(),
                name: "Filesystem Gene".into(),
                version: "1.0.0".into(),
                kind: PackageKind::Gene,
                description: "File system operations — read, write, list, search".into(),
                author: "openpandora".into(),
                license: "MIT".into(),
                trust: TrustInfo {
                    level: TrustLevel::Official,
                    signature: None,
                    content_hash: None,
                    publisher: "openpandora".into(),
                },
                capabilities: CapabilityInfo {
                    provides: vec!["filesystem.read".into(), "filesystem.write".into()],
                    requires: vec![],
                },
                downloads: 28930,
                success_rate: 0.99,
                compatibility: CompatibilityInfo {
                    runtimes: vec!["pandora>=0.2".into()],
                    platforms: vec!["linux".into(), "macos".into(), "windows".into()],
                },
                repository: Some("https://github.com/openpandora/filesystem-gene".into()),
                homepage: None,
                tags: vec!["filesystem".into(), "tool".into(), "infrastructure".into()],
                created_at: now,
                updated_at: now,
            },
            Package {
                id: "shell.gene".into(),
                name: "Shell Gene".into(),
                version: "1.0.0".into(),
                kind: PackageKind::Gene,
                description: "Shell command execution with permission gating".into(),
                author: "openpandora".into(),
                license: "MIT".into(),
                trust: TrustInfo {
                    level: TrustLevel::Official,
                    signature: None,
                    content_hash: None,
                    publisher: "openpandora".into(),
                },
                capabilities: CapabilityInfo {
                    provides: vec!["shell.execute".into()],
                    requires: vec![],
                },
                downloads: 22100,
                success_rate: 0.95,
                compatibility: CompatibilityInfo {
                    runtimes: vec!["pandora>=0.2".into()],
                    platforms: vec!["linux".into(), "macos".into(), "windows".into()],
                },
                repository: Some("https://github.com/openpandora/shell-gene".into()),
                homepage: None,
                tags: vec!["shell".into(), "tool".into(), "execution".into()],
                created_at: now,
                updated_at: now,
            },
            Package {
                id: "mcp.github".into(),
                name: "GitHub MCP Server".into(),
                version: "2.1.0".into(),
                kind: PackageKind::Connector,
                description: "GitHub MCP server — repos, issues, PRs, actions".into(),
                author: "modelcontextprotocol".into(),
                license: "MIT".into(),
                trust: TrustInfo {
                    level: TrustLevel::Verified,
                    signature: None,
                    content_hash: None,
                    publisher: "modelcontextprotocol".into(),
                },
                capabilities: CapabilityInfo {
                    provides: vec!["github.repos".into(), "github.issues".into(), "github.pulls".into()],
                    requires: vec![],
                },
                downloads: 45200,
                success_rate: 0.98,
                compatibility: CompatibilityInfo {
                    runtimes: vec!["pandora>=0.2".into(), "goose".into(), "cline".into()],
                    platforms: vec!["linux".into(), "macos".into(), "windows".into()],
                },
                repository: Some("https://github.com/modelcontextprotocol/servers".into()),
                homepage: None,
                tags: vec!["mcp".into(), "github".into(), "integration".into()],
                created_at: now,
                updated_at: now,
            },
            Package {
                id: "coding.pack".into(),
                name: "Rust Coding Pack".into(),
                version: "0.5.0".into(),
                kind: PackageKind::CapabilityPack,
                description: "Complete Rust development pack — shell, filesystem, git, cargo, rust-analyzer".into(),
                author: "openpandora".into(),
                license: "MIT".into(),
                trust: TrustInfo {
                    level: TrustLevel::Verified,
                    signature: None,
                    content_hash: None,
                    publisher: "openpandora".into(),
                },
                capabilities: CapabilityInfo {
                    provides: vec!["coding.rust".into()],
                    requires: vec!["shell.execute".into(), "filesystem.read".into()],
                },
                downloads: 8200,
                success_rate: 0.94,
                compatibility: CompatibilityInfo {
                    runtimes: vec!["pandora>=0.2".into()],
                    platforms: vec!["linux".into(), "macos".into(), "windows".into()],
                },
                repository: Some("https://github.com/openpandora/coding-pack".into()),
                homepage: None,
                tags: vec!["coding".into(), "rust".into(), "pack".into()],
                created_at: now,
                updated_at: now,
            },
            Package {
                id: "research.edition".into(),
                name: "Research Edition".into(),
                version: "0.1.0".into(),
                kind: PackageKind::Distribution,
                description: "Complete Pandora distribution for research — search, scrape, analyze, cite".into(),
                author: "openpandora".into(),
                license: "MIT".into(),
                trust: TrustInfo {
                    level: TrustLevel::Official,
                    signature: None,
                    content_hash: None,
                    publisher: "openpandora".into(),
                },
                capabilities: CapabilityInfo {
                    provides: vec!["distribution.research".into()],
                    requires: vec![],
                },
                downloads: 1200,
                success_rate: 0.91,
                compatibility: CompatibilityInfo {
                    runtimes: vec!["pandora>=0.2".into()],
                    platforms: vec!["linux".into(), "macos".into(), "windows".into()],
                },
                repository: Some("https://github.com/openpandora/research-edition".into()),
                homepage: None,
                tags: vec!["distribution".into(), "research".into()],
                created_at: now,
                updated_at: now,
            },
        ];

        for pkg in samples {
            self.packages.insert(pkg.id.clone(), pkg);
        }
    }
}
