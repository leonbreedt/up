CREATE TYPE user_type AS ENUM('SYSTEM', 'STANDARD');

CREATE TABLE IF NOT EXISTS users (
    id         BIGSERIAL PRIMARY KEY,
    uuid       UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid    TEXT NOT NULL,
    user_type  user_type NOT NULL DEFAULT 'STANDARD',
    -- the randomly generated subject is what goes into JWT, not identifying information like email.
    -- so that it can be revoked easily, invalidating any existing JWTs in the wild immediately.
    subject    TEXT NOT NULL,
    email      TEXT NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    created_by BIGINT REFERENCES users(id),
    updated_at TIMESTAMP WITHOUT TIME ZONE,
    updated_by BIGINT REFERENCES users(id),
    deleted    BOOLEAN NOT NULL DEFAULT false,
    deleted_at TIMESTAMP WITHOUT TIME ZONE,
    deleted_by BIGINT REFERENCES users(id),

    CONSTRAINT users_unique_uuid UNIQUE (uuid),
    CONSTRAINT users_unique_shortid UNIQUE (shortid),
    CONSTRAINT users_unique_subject UNIQUE (subject)
);

CREATE TABLE IF NOT EXISTS accounts (
    id         BIGSERIAL PRIMARY KEY,
    uuid       UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid    TEXT NOT NULL,
    name       TEXT NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    created_by BIGINT NOT NULL REFERENCES users(id),
    updated_at TIMESTAMP WITHOUT TIME ZONE,
    updated_by BIGINT REFERENCES users(id),
    deleted    BOOLEAN NOT NULL DEFAULT false,
    deleted_at TIMESTAMP WITHOUT TIME ZONE,
    deleted_by BIGINT REFERENCES users(id),

    CONSTRAINT accounts_unique_uuid UNIQUE (uuid),
    CONSTRAINT accounts_unique_shortid UNIQUE (shortid)
);

CREATE UNIQUE INDEX IF NOT EXISTS users_unique_email
    ON users (email)
    WHERE deleted = false;

CREATE TABLE IF NOT EXISTS user_accounts (
    user_id    BIGINT NOT NULL REFERENCES users (id),
    account_id BIGINT NOT NULL REFERENCES accounts (id),

    PRIMARY KEY (user_id, account_id)
);

CREATE TYPE user_role AS ENUM ('ADMINISTRATOR', 'MEMBER', 'VIEWER');

CREATE TABLE IF NOT EXISTS user_roles (
    user_id    BIGINT NOT NULL REFERENCES users (id),
    account_id BIGINT NOT NULL REFERENCES accounts (id),
    role       user_role NOT NULL,

    PRIMARY KEY (user_id, role)
);

CREATE TABLE IF NOT EXISTS projects (
    id         BIGSERIAL PRIMARY KEY,
    account_id BIGINT NOT NULL REFERENCES accounts (id),
    uuid       UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid    TEXT NOT NULL,
    name       TEXT NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    created_by BIGINT NOT NULL REFERENCES users(id),
    updated_at TIMESTAMP WITHOUT TIME ZONE,
    updated_by BIGINT REFERENCES users(id),
    deleted    BOOLEAN NOT NULL DEFAULT false,
    deleted_at TIMESTAMP WITHOUT TIME ZONE,
    deleted_by BIGINT REFERENCES users(id),

    CONSTRAINT projects_unique_uuid UNIQUE (uuid),
    CONSTRAINT projects_unique_shortid UNIQUE (shortid)
);

CREATE UNIQUE INDEX IF NOT EXISTS projects_unique_name_account_id
    ON projects (name, account_id)
    WHERE deleted = false;

CREATE TABLE IF NOT EXISTS user_projects (
    user_id    BIGINT NOT NULL REFERENCES users (id),
    project_id BIGINT NOT NULL REFERENCES projects (id),

    PRIMARY KEY (user_id, project_id)
);

CREATE TYPE schedule_type AS ENUM ('SIMPLE', 'CRON');
CREATE TYPE period_units  AS ENUM ('MINUTES', 'HOURS', 'DAYS');
CREATE TYPE check_status  AS ENUM ('UP', 'DOWN', 'CREATED', 'PAUSED');

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
    created_by           BIGINT NOT NULL REFERENCES users(id),
    updated_at           TIMESTAMP WITHOUT TIME ZONE,
    updated_by           BIGINT REFERENCES users(id),
    deleted              BOOLEAN NOT NULL DEFAULT false,
    deleted_at           TIMESTAMP WITHOUT TIME ZONE,
    deleted_by           BIGINT REFERENCES users(id),

    CONSTRAINT checks_unique_uuid UNIQUE (uuid),
    CONSTRAINT checks_unique_ping_key UNIQUE (ping_key),
    CONSTRAINT checks_unique_shortid UNIQUE (shortid)
);

CREATE UNIQUE INDEX IF NOT EXISTS checks_unique_name_account_project_id
    ON checks (name, account_id, project_id)
    WHERE deleted = false;

CREATE TYPE notification_type AS ENUM ('EMAIL', 'WEBHOOK');

CREATE TABLE IF NOT EXISTS notifications (
    id                BIGSERIAL PRIMARY KEY,
    account_id        BIGINT NOT NULL REFERENCES accounts (id),
    project_id        BIGINT NOT NULL REFERENCES projects (id),
    check_id          BIGINT NOT NULL REFERENCES checks (id),
    uuid              UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid           TEXT NOT NULL,
    name              TEXT NOT NULL DEFAULT '',
    notification_type notification_type NOT NULL,
    email             TEXT,
    url               TEXT,
    max_retries       INTEGER NOT NULL DEFAULT 5,
    created_at        TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    created_by        BIGINT NOT NULL REFERENCES users(id),
    updated_at        TIMESTAMP WITHOUT TIME ZONE,
    updated_by        BIGINT REFERENCES users(id),
    deleted           BOOLEAN NOT NULL DEFAULT false,
    deleted_at        TIMESTAMP WITHOUT TIME ZONE,
    deleted_by        BIGINT REFERENCES users(id),

    CONSTRAINT notifications_unique_uuid UNIQUE (uuid),
    CONSTRAINT notifications_unique_shortid UNIQUE (shortid)
);

CREATE TYPE alert_delivery_status AS ENUM ('QUEUED', 'RUNNING', 'DELIVERED', 'FAILED');

CREATE TABLE IF NOT EXISTS notification_alerts (
    id                  BIGSERIAL PRIMARY KEY,
    notification_id     BIGINT NOT NULL REFERENCES notifications (id),
    check_status        check_status NOT NULL,
    delivery_status     alert_delivery_status NOT NULL DEFAULT 'QUEUED',
    retries_remaining   INTEGER NOT NULL,
    created_at          TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    finished_at         TIMESTAMP WITHOUT TIME ZONE
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
