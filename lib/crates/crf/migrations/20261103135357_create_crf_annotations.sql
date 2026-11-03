-- crf_annotations: polymorphic — owned by exactly one of form / item / option / unit.
-- The CHECK constraint enforces the "exactly one of the four FKs is non-null" invariant.

CREATE TABLE IF NOT EXISTS crf_annotations (
    id                   BIGSERIAL PRIMARY KEY,
    form_id              BIGINT REFERENCES crf_forms(id)    ON DELETE CASCADE,
    item_id              BIGINT REFERENCES crf_items(id)    ON DELETE CASCADE,
    option_id            BIGINT REFERENCES crf_options(id)  ON DELETE CASCADE,
    unit_id              BIGINT REFERENCES crf_units(id)    ON DELETE CASCADE,
    domain_annotation_id BIGINT NOT NULL REFERENCES crf_domain_annotations(id) ON DELETE RESTRICT,
    content              TEXT NOT NULL,
    assign               BOOLEAN NOT NULL DEFAULT FALSE,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT crf_annotations_polymorphic_owner CHECK (
        (form_id IS NOT NULL)::int + (item_id IS NOT NULL)::int
      + (option_id IS NOT NULL)::int + (unit_id IS NOT NULL)::int = 1
    )
);

CREATE OR REPLACE FUNCTION crf_annotations_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER crf_annotations_updated_at
BEFORE UPDATE ON crf_annotations
FOR EACH ROW EXECUTE FUNCTION crf_annotations_set_updated_at();
