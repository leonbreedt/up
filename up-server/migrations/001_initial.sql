CREATE TABLE IF NOT EXISTS accounts (
    id          BIGSERIAL PRIMARY KEY,
    uuid        UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid     TEXT NOT NULL,
    name        TEXT NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  TIMESTAMP WITHOUT TIME ZONE,
    CONSTRAINT  accounts_unique_uuid UNIQUE (uuid),
    CONSTRAINT  accounts_unique_shortid UNIQUE (shortid)
);

CREATE TABLE IF NOT EXISTS users (
    id          BIGSERIAL PRIMARY KEY,
    uuid        UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid     TEXT NOT NULL,
    email       TEXT NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  TIMESTAMP WITHOUT TIME ZONE,
    CONSTRAINT  users_unique_uuid UNIQUE (uuid),
    CONSTRAINT  users_unique_shortid UNIQUE (shortid)
);

CREATE TABLE IF NOT EXISTS user_accounts (
    user_id    BIGINT NOT NULL REFERENCES users (id),
    account_id BIGINT NOT NULL REFERENCES accounts (id),
    PRIMARY KEY (user_id, account_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS users_unique_email
ON users (email)
WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS projects (
    id          BIGSERIAL PRIMARY KEY,
    account_id  BIGINT NOT NULL REFERENCES accounts (id),
    uuid        UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid     TEXT NOT NULL,
    name        TEXT NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  TIMESTAMP WITHOUT TIME ZONE,
    CONSTRAINT  projects_unique_uuid UNIQUE (uuid),
    CONSTRAINT  projects_unique_shortid UNIQUE (shortid)
);

CREATE TABLE IF NOT EXISTS user_projects (
    user_id    BIGINT NOT NULL REFERENCES users (id),
    project_id BIGINT NOT NULL REFERENCES projects (id),
    PRIMARY KEY (user_id, project_id)
);

CREATE TYPE schedule_type AS ENUM ('SIMPLE', 'CRON');
CREATE TYPE period_units  AS ENUM ('MINUTES', 'HOURS', 'DAYS');
CREATE TYPE check_status  AS ENUM ('UP', 'DOWN', 'CREATED');

CREATE TABLE IF NOT EXISTS checks (
    id                   BIGSERIAL PRIMARY KEY,
    account_id           BIGINT NOT NULL REFERENCES accounts (id),
    project_id           BIGINT NOT NULL REFERENCES projects (id),
    uuid                 UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid              TEXT NOT NULL,
    ping_key             TEXT NOT NULL,
    name                 TEXT NOT NULL DEFAULT '',
    description          TEXT NOT NULL DEFAULT '',
    schedule_type        schedule_type NOT NULL DEFAULT 'SIMPLE',
    ping_period          INTEGER DEFAULT 1,
    ping_period_units    period_units DEFAULT 'DAYS',
    ping_cron_expression TEXT,
    grace_period         INTEGER NOT NULL DEFAULT 1,
    grace_period_units   period_units NOT NULL DEFAULT 'HOURS',
    status               check_status NOT NULL DEFAULT 'CREATED',
    last_ping_at         TIMESTAMP WITHOUT TIME ZONE,
    created_at           TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at           TIMESTAMP WITHOUT TIME ZONE,
    deleted              BOOLEAN NOT NULL DEFAULT false,
    deleted_at           TIMESTAMP WITHOUT TIME ZONE,

    CONSTRAINT checks_unique_uuid UNIQUE (uuid),
    CONSTRAINT checks_unique_ping_key UNIQUE (ping_key),
    CONSTRAINT checks_unique_shortid UNIQUE (shortid)
);

CREATE TABLE IF NOT EXISTS tags (
    id          BIGSERIAL PRIMARY KEY,
    account_id  BIGINT NOT NULL REFERENCES accounts (id),
    name        TEXT NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    CONSTRAINT  tags_unique_uuid UNIQUE (name)
);

CREATE TABLE IF NOT EXISTS check_tags (
    check_id BIGINT NOT NULL REFERENCES checks (id),
    tag_id   BIGINT NOT NULL REFERENCES tags (id),
    PRIMARY KEY (tag_id, check_id)
);
