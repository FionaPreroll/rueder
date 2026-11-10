-- The schema of the old rueder, as its migrations in backend/migrations/ (Fizz)
-- create it on PostgreSQL. The import is tested against this.

CREATE TABLE feeds (
    id            uuid          NOT NULL PRIMARY KEY,
    created_at    timestamp     NOT NULL,
    updated_at    timestamp     NOT NULL,
    fetched_at    timestamp     NOT NULL,
    fetch_delay_s integer       NOT NULL,
    fetcher_state jsonb         NOT NULL,
    feed_url      varchar(2048) NOT NULL,
    site_url      varchar(2048),
    title         varchar(1024),
    icon          varchar(2048)
);

CREATE TABLE articles (
    id          uuid          NOT NULL PRIMARY KEY,
    created_at  timestamp     NOT NULL,
    updated_at  timestamp     NOT NULL,
    seq         bigserial     NOT NULL,
    feed_id     uuid          NOT NULL REFERENCES feeds (id) ON DELETE CASCADE,
    site_guid   varchar(2048) NOT NULL,
    posted_at   timestamp     NOT NULL,
    link        varchar(2048),
    thumbnail   varchar(2048),
    image       varchar(2048),
    image_title varchar(2048),
    title       text,
    teaser      text,
    content     jsonb         NOT NULL
);

CREATE UNIQUE INDEX articles_site_guid_feed_id_idx ON articles (site_guid, feed_id);
CREATE UNIQUE INDEX articles_seq_idx ON articles (seq);
CREATE INDEX articles_feed_id_idx ON articles (feed_id);

CREATE TABLE labels (
    id         uuid      NOT NULL PRIMARY KEY,
    created_at timestamp NOT NULL,
    updated_at timestamp NOT NULL,
    title      varchar(1024),
    color      varchar(16)
);

CREATE TABLE users (
    id           uuid         NOT NULL PRIMARY KEY,
    created_at   timestamp    NOT NULL,
    updated_at   timestamp    NOT NULL,
    auth_origin  varchar(255) NOT NULL,
    auth_subject varchar(255) NOT NULL,
    folders      jsonb        NOT NULL
);

CREATE UNIQUE INDEX users_auth_origin_auth_subject_idx ON users (auth_origin, auth_subject);

CREATE TABLE user_feeds (
    id      serial NOT NULL,
    user_id uuid   NOT NULL,
    feed_id uuid   NOT NULL,
    PRIMARY KEY (user_id, feed_id)
);

CREATE TABLE user_states (
    id          uuid      NOT NULL,
    created_at  timestamp NOT NULL,
    updated_at  timestamp NOT NULL,
    user_id     uuid      NOT NULL,
    feed_states jsonb     NOT NULL,
    PRIMARY KEY (id, user_id)
);
