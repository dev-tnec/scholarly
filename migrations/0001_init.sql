-- Scholarly schema, PRD-rust.md section 5. Status is computed, never stored.
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE scholarships (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    slug TEXT UNIQUE NOT NULL CHECK (slug ~ '^[a-z0-9-]+$'),
    name TEXT NOT NULL,
    official_name TEXT NOT NULL,
    abbreviation TEXT NOT NULL DEFAULT '',
    provider TEXT NOT NULL,
    provider_type TEXT NOT NULL CHECK (provider_type IN ('Pemerintah', 'Kampus', 'Swasta', 'Yayasan')),
    location TEXT NOT NULL CHECK (location IN ('Dalam Negeri', 'Luar Negeri')),
    country TEXT NOT NULL,
    levels TEXT[] NOT NULL,
    funding_type TEXT NOT NULL CHECK (funding_type IN ('Fully Funded', 'Parsial', 'Tunjangan')),
    open_date DATE,
    close_date DATE,
    deadline_type TEXT NOT NULL CHECK (deadline_type IN ('fixed', 'rolling', 'unknown')),
    official_url TEXT NOT NULL CHECK (official_url LIKE 'https://%'),
    checked_at DATE NOT NULL CHECK (checked_at <= CURRENT_DATE),
    quota TEXT NOT NULL DEFAULT '',
    duration TEXT NOT NULL DEFAULT '',
    language_requirement TEXT NOT NULL DEFAULT '',
    visa_requirement TEXT NOT NULL DEFAULT '',
    research_proposal TEXT NOT NULL DEFAULT '',
    title_tag TEXT NOT NULL,
    meta_description TEXT NOT NULL,
    notes TEXT NOT NULL DEFAULT '',
    popularity INTEGER NOT NULL DEFAULT 0,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT close_not_before_open CHECK (close_date IS NULL OR open_date IS NULL OR close_date >= open_date)
);

CREATE TABLE benefits (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    scholarship_id BIGINT NOT NULL REFERENCES scholarships(id) ON DELETE CASCADE,
    item TEXT NOT NULL,
    detail TEXT NOT NULL,
    source_url TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE requirements (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    scholarship_id BIGINT NOT NULL REFERENCES scholarships(id) ON DELETE CASCADE,
    group_name TEXT NOT NULL CHECK (group_name IN ('Syarat Umum', 'Syarat Akademik')),
    text TEXT NOT NULL,
    source_url TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE documents (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    scholarship_id BIGINT NOT NULL REFERENCES scholarships(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    required INTEGER NOT NULL DEFAULT 1 CHECK (required IN (0, 1)),
    source_url TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE steps (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    scholarship_id BIGINT NOT NULL REFERENCES scholarships(id) ON DELETE CASCADE,
    step_no INTEGER NOT NULL,
    title TEXT NOT NULL,
    detail TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE timeline_events (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    scholarship_id BIGINT NOT NULL REFERENCES scholarships(id) ON DELETE CASCADE,
    stage TEXT NOT NULL,
    event_date DATE,
    event_label TEXT NOT NULL DEFAULT '',
    note TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE faqs (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    scholarship_id BIGINT NOT NULL REFERENCES scholarships(id) ON DELETE CASCADE,
    question TEXT NOT NULL,
    answer TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE related (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    scholarship_id BIGINT NOT NULL REFERENCES scholarships(id) ON DELETE CASCADE,
    related_id BIGINT NOT NULL REFERENCES scholarships(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT not_self CHECK (related_id <> scholarship_id)
);

CREATE TABLE redirects (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    from_path TEXT UNIQUE NOT NULL,
    to_path TEXT NOT NULL,
    status_code INTEGER NOT NULL DEFAULT 301,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE import_runs (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    started_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ,
    source_file TEXT NOT NULL,
    rows_ok INTEGER NOT NULL DEFAULT 0,
    rows_failed INTEGER NOT NULL DEFAULT 0,
    report_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Trigram search over the demo's searchable fields, PRD section 5.
CREATE INDEX idx_scholarships_name_trgm ON scholarships USING gin (name gin_trgm_ops);
CREATE INDEX idx_scholarships_official_name_trgm ON scholarships USING gin (official_name gin_trgm_ops);
CREATE INDEX idx_scholarships_provider_trgm ON scholarships USING gin (provider gin_trgm_ops);
CREATE INDEX idx_scholarships_country_trgm ON scholarships USING gin (country gin_trgm_ops);

CREATE INDEX idx_benefits_scholarship ON benefits (scholarship_id);
CREATE INDEX idx_requirements_scholarship ON requirements (scholarship_id);
CREATE INDEX idx_documents_scholarship ON documents (scholarship_id);
CREATE INDEX idx_steps_scholarship ON steps (scholarship_id);
CREATE INDEX idx_timeline_scholarship ON timeline_events (scholarship_id);
CREATE INDEX idx_faqs_scholarship ON faqs (scholarship_id);
CREATE INDEX idx_related_scholarship ON related (scholarship_id);
