import { useEffect, useMemo, useState } from "react";
import { createPalaceClient, PalaceApiError } from "./api/palaceClient";
import { demoPackages } from "./data/demoPackages";
import {
  formatDownloads,
  formatLabel,
  formatSuccessRate,
  getCompatibilityOptions,
  getLicenseOptions,
  hasSignature,
  matchesMode,
  relativeFreshness,
  trustLevels,
  type DiscoveryMode,
  type SignatureFilter,
} from "./lib/catalog";
import type { Package } from "./types";

type Theme = "light" | "dark";
type DataSource = "live" | "preview" | "empty";

const palaceClient = createPalaceClient();

const readStoredTheme = (): Theme => {
  const storedTheme = window.localStorage.getItem("ko-palace-theme");
  return storedTheme === "light" ? "light" : "dark";
};

const copyByMode: Record<DiscoveryMode, { eyebrow: string; body: string }> = {
  pandora: {
    eyebrow: "Pandora Mode",
    body: "Genes, harnesses, skills, MCP-style connectors, and integrations that advertise Pandora compatibility.",
  },
  agent: {
    eyebrow: "Agent Mode",
    body: "Adapter and discovery cards for external runtimes. This view never launches, shells into, or executes agents.",
  },
};

function App() {
  const [theme, setTheme] = useState<Theme>(readStoredTheme);
  const [mode, setMode] = useState<DiscoveryMode>("pandora");
  const [query, setQuery] = useState("pandora");
  const [selectedTrusts, setSelectedTrusts] = useState<string[]>(["official", "verified", "community"]);
  const [signatureFilter, setSignatureFilter] = useState<SignatureFilter>("all");
  const [selectedLicense, setSelectedLicense] = useState("all");
  const [selectedRuntime, setSelectedRuntime] = useState("all");
  const [packages, setPackages] = useState<Package[]>(demoPackages);
  const [liveTotal, setLiveTotal] = useState<number>(demoPackages.length);
  const [dataSource, setDataSource] = useState<DataSource>("preview");
  const [loading, setLoading] = useState(false);
  const [statusMessage, setStatusMessage] = useState(
    "Showing a local preview until the registry responds.",
  );

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    window.localStorage.setItem("ko-palace-theme", theme);
  }, [theme]);

  useEffect(() => {
    const trimmedQuery = query.trim();

    if (!trimmedQuery) {
      setPackages(demoPackages);
      setLiveTotal(demoPackages.length);
      setDataSource("preview");
      setStatusMessage("Add a query to search the live registry. Preview cards stay local.");
      setLoading(false);
      return;
    }

    const controller = new AbortController();
    setLoading(true);

    palaceClient
      .searchPackages({
        q: trimmedQuery,
        limit: 24,
        signal: controller.signal,
      })
      .then((response) => {
        setPackages(response.packages);
        setLiveTotal(response.total);
        setDataSource(response.packages.length > 0 ? "live" : "empty");
        setStatusMessage(
          response.packages.length > 0
            ? `Live registry results for “${trimmedQuery}”.`
            : `No live package matches for “${trimmedQuery}”.`,
        );
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) {
          return;
        }

        const fallbackMessage =
          error instanceof PalaceApiError
            ? error.message
            : "Registry search is unavailable right now.";

        setPackages(demoPackages);
        setLiveTotal(demoPackages.length);
        setDataSource("preview");
        setStatusMessage(`${fallbackMessage} Showing local preview cards instead.`);
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      });

    return () => controller.abort();
  }, [query]);

  const runtimeOptions = useMemo(() => getCompatibilityOptions(packages), [packages]);
  const licenseOptions = useMemo(() => getLicenseOptions(packages), [packages]);

  const filteredPackages = useMemo(() => {
    return packages
      .filter((pkg) => matchesMode(pkg, mode))
      .filter((pkg) => selectedTrusts.includes(pkg.trust.level))
      .filter((pkg) => {
        if (signatureFilter === "signed") {
          return hasSignature(pkg);
        }

        if (signatureFilter === "unsigned") {
          return !hasSignature(pkg);
        }

        return true;
      })
      .filter((pkg) => (selectedLicense === "all" ? true : pkg.license === selectedLicense))
      .filter((pkg) =>
        selectedRuntime === "all"
          ? true
          : pkg.compatibility.runtimes.some((runtime) => runtime === selectedRuntime),
      );
  }, [mode, packages, selectedLicense, selectedRuntime, selectedTrusts, signatureFilter]);

  const signedCount = filteredPackages.filter(hasSignature).length;
  const averageSuccessRate = filteredPackages.length
    ? filteredPackages.reduce((sum, pkg) => sum + pkg.success_rate, 0) / filteredPackages.length
    : 0;

  const toggleTrust = (trustLevel: string) => {
    setSelectedTrusts((currentTrusts) => {
      if (currentTrusts.includes(trustLevel)) {
        return currentTrusts.length === 1
          ? currentTrusts
          : currentTrusts.filter((item) => item !== trustLevel);
      }

      return [...currentTrusts, trustLevel];
    });
  };

  return (
    <div className="app-shell">
      <header className="bento hero-card hero-grid">
        <div className="hero-copy">
          <span className="eyebrow">K-O Palace</span>
          <h1>Package discovery with trust signals, not surprise execution.</h1>
          <p className="hero-body">
            Review compatibility, signatures, and publisher trust before any install decision.
            This client stays read-only.
          </p>
          <div className="hero-actions">
            <label className="search-field" htmlFor="package-search">
              <span className="sr-only">Search packages</span>
              <input
                id="package-search"
                name="package-search"
                type="search"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Search genes, harnesses, adapters, or runtimes"
              />
            </label>
            <button
              type="button"
              className="theme-toggle"
              onClick={() => setTheme((currentTheme) => (currentTheme === "dark" ? "light" : "dark"))}
            >
              {theme === "dark" ? "Light theme" : "Dark theme"}
            </button>
          </div>
        </div>

        <div className="hero-side">
          <div className="mini-card">
            <span className="mini-label">Registry source</span>
            <strong>{dataSource === "live" ? "Live API" : dataSource === "empty" ? "No matches" : "Local preview"}</strong>
            <p>{statusMessage}</p>
          </div>
          <div className="mini-card notice-card">
            <span className="mini-label">Safety boundary</span>
            <strong>Never auto-executes packages</strong>
            <p>Discovery only. Install flow, sandbox policy, and runtime execution stay outside this UI.</p>
          </div>
        </div>
      </header>

      <main className="bento-grid">
        <section className="bento mode-card">
          <div className="section-heading">
            <span className="eyebrow">{copyByMode[mode].eyebrow}</span>
            <h2>Mode focus</h2>
          </div>
          <p>{copyByMode[mode].body}</p>
          <div className="mode-toggle" role="tablist" aria-label="Discovery mode">
            <button
              type="button"
              className={mode === "pandora" ? "active" : ""}
              onClick={() => setMode("pandora")}
            >
              Pandora Mode
            </button>
            <button
              type="button"
              className={mode === "agent" ? "active" : ""}
              onClick={() => setMode("agent")}
            >
              Agent Mode
            </button>
          </div>
        </section>

        <section className="bento stats-card">
          <div className="section-heading">
            <span className="eyebrow">At a glance</span>
            <h2>Trust snapshot</h2>
          </div>
          <div className="stats-grid">
            <div className="stat">
              <span>Visible packages</span>
              <strong>{filteredPackages.length}</strong>
            </div>
            <div className="stat">
              <span>Signed</span>
              <strong>{signedCount}</strong>
            </div>
            <div className="stat">
              <span>Success rate</span>
              <strong>{formatSuccessRate(averageSuccessRate)}</strong>
            </div>
            <div className="stat">
              <span>Live total</span>
              <strong>{liveTotal}</strong>
            </div>
          </div>
        </section>

        <section className="bento filters-card">
          <div className="section-heading">
            <span className="eyebrow">Refine</span>
            <h2>Trust and compatibility filters</h2>
          </div>

          <div className="filter-block">
            <span className="filter-label">Trust</span>
            <div className="chip-row">
              {trustLevels.map((trustLevel) => (
                <button
                  key={trustLevel}
                  type="button"
                  className={selectedTrusts.includes(trustLevel) ? "chip active" : "chip"}
                  onClick={() => toggleTrust(trustLevel)}
                >
                  {formatLabel(trustLevel)}
                </button>
              ))}
            </div>
          </div>

          <div className="filter-grid">
            <label className="select-field">
              <span>Signature</span>
              <select
                value={signatureFilter}
                onChange={(event) => setSignatureFilter(event.target.value as SignatureFilter)}
              >
                <option value="all">All packages</option>
                <option value="signed">Signed only</option>
                <option value="unsigned">Unsigned only</option>
              </select>
            </label>

            <label className="select-field">
              <span>License</span>
              <select
                value={selectedLicense}
                onChange={(event) => setSelectedLicense(event.target.value)}
              >
                <option value="all">All licenses</option>
                {licenseOptions.map((license) => (
                  <option key={license} value={license}>
                    {license}
                  </option>
                ))}
              </select>
            </label>

            <label className="select-field">
              <span>Compatibility</span>
              <select
                value={selectedRuntime}
                onChange={(event) => setSelectedRuntime(event.target.value)}
              >
                <option value="all">All runtimes</option>
                {runtimeOptions.map((runtime) => (
                  <option key={runtime} value={runtime}>
                    {runtime}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </section>

        <section className="bento results-card">
          <div className="results-header">
            <div>
              <span className="eyebrow">Discovery</span>
              <h2>Package cards</h2>
            </div>
            <span className="results-note">
              {loading ? "Refreshing search…" : statusMessage}
            </span>
          </div>

          {filteredPackages.length === 0 ? (
            <div className="empty-state">
              <h3>No packages match this view.</h3>
              <p>Try loosening trust filters, changing runtimes, or switching modes.</p>
            </div>
          ) : (
            <div className="package-grid">
              {filteredPackages.map((pkg) => (
                <article key={`${pkg.id}@${pkg.version}`} className="package-card">
                  <div className="card-top">
                    <div>
                      <div className="title-row">
                        <h3>{pkg.name}</h3>
                        <span className="version-tag">{pkg.version}</span>
                      </div>
                      <p className="package-id">{pkg.id}</p>
                    </div>
                    <span className={`trust-badge trust-${pkg.trust.level}`}>{formatLabel(pkg.trust.level)}</span>
                  </div>

                  <p className="package-description">{pkg.description}</p>

                  <div className="meta-row">
                    <span>{formatLabel(pkg.kind)}</span>
                    <span>{pkg.license}</span>
                    <span>{formatDownloads(pkg.downloads)} downloads</span>
                    <span>{formatSuccessRate(pkg.success_rate)} success</span>
                  </div>

                  <div className="chip-row compact">
                    {pkg.compatibility.runtimes.map((runtime) => (
                      <span key={runtime} className="chip subtle">
                        {runtime}
                      </span>
                    ))}
                  </div>

                  <div className="signal-grid">
                    <div>
                      <span className="signal-label">Signature</span>
                      <strong>{hasSignature(pkg) ? "Present" : "Not published"}</strong>
                    </div>
                    <div>
                      <span className="signal-label">Publisher</span>
                      <strong>{pkg.trust.publisher}</strong>
                    </div>
                    <div>
                      <span className="signal-label">Platforms</span>
                      <strong>{pkg.compatibility.platforms.join(", ") || "Not listed"}</strong>
                    </div>
                    <div>
                      <span className="signal-label">Freshness</span>
                      <strong>{relativeFreshness(pkg.updated_at)}</strong>
                    </div>
                  </div>

                  <div className="chip-row compact">
                    {pkg.tags.map((tag) => (
                      <span key={tag} className="chip subtle">
                        #{tag}
                      </span>
                    ))}
                  </div>

                  <div className="capability-block">
                    <div>
                      <span className="signal-label">Provides</span>
                      <p>{pkg.capabilities.provides.join(", ") || "None listed"}</p>
                    </div>
                    <div>
                      <span className="signal-label">Requires</span>
                      <p>{pkg.capabilities.requires.join(", ") || "None listed"}</p>
                    </div>
                  </div>

                  <div className="card-actions">
                    {pkg.repository ? (
                      <a href={pkg.repository} target="_blank" rel="noreferrer">
                        Repository
                      </a>
                    ) : null}
                    {pkg.homepage ? (
                      <a href={pkg.homepage} target="_blank" rel="noreferrer">
                        Homepage
                      </a>
                    ) : null}
                    {pkg.artifact_url ? (
                      <a href={pkg.artifact_url} target="_blank" rel="noreferrer">
                        Artifact
                      </a>
                    ) : null}
                  </div>
                </article>
              ))}
            </div>
          )}
        </section>
      </main>
    </div>
  );
}

export default App;
