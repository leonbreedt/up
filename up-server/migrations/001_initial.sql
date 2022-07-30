CREATE TABLE IF NOT EXISTS accounts (
    id          BIGSERIAL PRIMARY KEY,
    uuid        UUID NOT NULL DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  TIMESTAMP WITHOUT TIME ZONE
);

CREATE TABLE IF NOT EXISTS projects (
    id          BIGSERIAL PRIMARY KEY,
    account_id  BIGINT NOT NULL REFERENCES accounts (id),
    uuid        UUID NOT NULL DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  TIMESTAMP WITHOUT TIME ZONE
);

CREATE TABLE IF NOT EXISTS checks (
    id          BIGSERIAL PRIMARY KEY,
    account_id  BIGINT NOT NULL REFERENCES accounts (id),
    project_id  BIGINT NOT NULL REFERENCES projects (id),
    uuid        UUID NOT NULL DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  TIMESTAMP WITHOUT TIME ZONE
);
