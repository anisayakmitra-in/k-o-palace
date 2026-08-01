CREATE INDEX IF NOT EXISTS idx_artifacts_package_version
    ON artifacts(package_id, package_version);

CREATE INDEX IF NOT EXISTS idx_signatures_package_version_verified
    ON signatures(package_id, package_version)
    WHERE verified_at IS NOT NULL;