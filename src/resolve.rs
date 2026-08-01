use crate::models::{CompatibilityInfo, Package};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

const MIN_RESOLUTION_BUDGET: usize = 64;
const MAX_RESOLUTION_STEPS: usize = 1_024;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolveOptions {
    pub runtime: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResponse {
    pub root: Package,
    pub options: ResolveOptions,
    pub root_compatibility: CompatibilityInfo,
    pub complete: bool,
    pub resolved_dependencies: Vec<ResolvedDependency>,
    pub missing_capabilities: Vec<String>,
    pub incompatible_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDependency {
    pub capability: String,
    pub selected_package_id: Option<String>,
    pub selected_version: Option<String>,
    pub selected_trust: Option<String>,
    pub reason: String,
}

pub fn resolve_dependencies(
    root: &Package,
    packages: &[Package],
    options: ResolveOptions,
) -> ResolutionResponse {
    let mut response = ResolutionResponse {
        root: root.clone(),
        options: options.clone(),
        root_compatibility: root.compatibility.clone(),
        complete: true,
        resolved_dependencies: Vec::new(),
        missing_capabilities: Vec::new(),
        incompatible_capabilities: Vec::new(),
    };

    let mut queue: VecDeque<String> = root
        .capabilities
        .requires
        .iter()
        .filter(|capability| !capability.trim().is_empty())
        .cloned()
        .collect();
    let mut seen_capabilities = HashSet::new();
    let mut expanded_packages = HashSet::new();
    let providers = provider_index(packages);
    let budget = resolution_budget(root);
    let mut steps = 0usize;

    while let Some(capability) = queue.pop_front() {
        if !seen_capabilities.insert(capability.clone()) {
            continue;
        }

        if steps >= budget || steps >= MAX_RESOLUTION_STEPS {
            response.complete = false;
            response.resolved_dependencies.push(ResolvedDependency {
                capability,
                selected_package_id: None,
                selected_version: None,
                selected_trust: None,
                reason: "resolution work budget exceeded".into(),
            });
            continue;
        }
        steps += 1;

        let candidates = providers
            .get(capability.as_str())
            .cloned()
            .unwrap_or_default();

        if candidates.is_empty() {
            response.complete = false;
            response.missing_capabilities.push(capability.clone());
            response.resolved_dependencies.push(ResolvedDependency {
                capability,
                selected_package_id: None,
                selected_version: None,
                selected_trust: None,
                reason: "no non-yanked package provides capability".into(),
            });
            continue;
        }

        let compatible_candidates: Vec<&Package> = candidates
            .into_iter()
            .filter(|package| matches_options(package, &options))
            .collect();

        if compatible_candidates.is_empty() {
            response.complete = false;
            response.incompatible_capabilities.push(capability.clone());
            response.resolved_dependencies.push(ResolvedDependency {
                capability,
                selected_package_id: None,
                selected_version: None,
                selected_trust: None,
                reason: "providers exist but none match the requested runtime/platform".into(),
            });
            continue;
        }

        let selected = compatible_candidates
            .into_iter()
            .max_by(compare_candidates)
            .expect("compatible_candidates is never empty");

        response.resolved_dependencies.push(ResolvedDependency {
            capability: capability.clone(),
            selected_package_id: Some(selected.id.clone()),
            selected_version: Some(selected.version.clone()),
            selected_trust: Some(selected.trust.level.as_str().to_string()),
            reason: "selected best compatible candidate".into(),
        });

        let package_key = format!("{}@{}", selected.id, selected.version);
        if expanded_packages.insert(package_key) {
            for required_capability in selected.capabilities.requires.iter() {
                if !required_capability.trim().is_empty() {
                    queue.push_back(required_capability.clone());
                }
            }
        }
    }

    response
}

fn resolution_budget(root: &Package) -> usize {
    let root_requirements = root.capabilities.requires.len();
    root_requirements.max(MIN_RESOLUTION_BUDGET)
}

fn provider_index(packages: &[Package]) -> HashMap<&str, Vec<&Package>> {
    let mut providers = HashMap::new();
    for package in packages.iter().filter(|package| !package.yanked) {
        for capability in &package.capabilities.provides {
            if !capability.trim().is_empty() {
                providers
                    .entry(capability.as_str())
                    .or_insert_with(Vec::new)
                    .push(package);
            }
        }
    }
    providers
}

fn matches_options(package: &Package, options: &ResolveOptions) -> bool {
    matches_dimension(options.runtime.as_deref(), &package.compatibility.runtimes)
        && matches_dimension(
            options.platform.as_deref(),
            &package.compatibility.platforms,
        )
}

fn matches_dimension(requested: Option<&str>, supported: &[String]) -> bool {
    match requested {
        None => true,
        Some(_) if supported.is_empty() => true,
        Some(requested_value) => {
            let requested_name = compatibility_name(requested_value);
            supported
                .iter()
                .map(|value| compatibility_name(value))
                .any(|value| value.eq_ignore_ascii_case(requested_name))
        }
    }
}

fn compatibility_name(value: &str) -> &str {
    let value = value.trim();
    let end = value
        .find(['<', '>', '=', '~', '^', '@', ' '])
        .unwrap_or(value.len());
    value[..end].trim_end()
}

fn compare_candidates(left: &&Package, right: &&Package) -> Ordering {
    left.trust
        .level
        .rank()
        .cmp(&right.trust.level.rank())
        .then_with(|| compare_versions(&left.version, &right.version))
        .then_with(|| left.downloads.cmp(&right.downloads))
        .then_with(|| right.id.cmp(&left.id))
        .then_with(|| left.version.cmp(&right.version))
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    match (Version::parse(left), Version::parse(right)) {
        (Ok(left_version), Ok(right_version)) => left_version.cmp(&right_version),
        (Ok(_), Err(_)) => Ordering::Greater,
        (Err(_), Ok(_)) => Ordering::Less,
        (Err(_), Err(_)) => Ordering::Equal,
    }
}
