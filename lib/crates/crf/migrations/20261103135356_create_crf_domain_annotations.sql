-- crf_domain_annotations: version-scoped label pool owned by exactly one form.

CREATE TABLE IF NOT EXISTS crf_domain_annotations (
    id          BIGSERIAL PRIMARY KEY,
    form_id     BIGINT NOT NULL REFERENCES crf_forms(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT crf_domain_annotations_form_name_unique UNIQUE (form_id, name)
);

CREATE OR REPLACE FUNCTION crf_domain_annotations_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER crf_domain_annotations_updated_at
BEFORE UPDATE ON crf_domain_annotations
FOR EACH ROW EXECUTE FUNCTION crf_domain_annotations_set_updated_at();
