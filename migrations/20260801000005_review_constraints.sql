-- Enforce one review per publisher and remove pre-existing duplicates before indexing.
DELETE FROM reviews AS duplicate
USING reviews AS original
WHERE duplicate.package_id = original.package_id
  AND duplicate.publisher_id = original.publisher_id
  AND duplicate.id > original.id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_reviews_package_publisher
    ON reviews(package_id, publisher_id);