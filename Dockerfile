# The new rueder as one image: the server (server/) with the built frontend
# (web/) inside. It runs next to PostgreSQL; deploy/compose.yaml starts both,
# and deploy/README.md says how to install it.
#
#   docker build -t rueder .
#
# Every base image is named by its tag and pinned to its digest, so that a tag
# moved by somebody else cannot change what is built; Dependabot raises both.

# The frontend: static files the server serves.
FROM node:22-trixie-slim@sha256:68c76cfad89921b927eae28ba21407475e29d0ce450afcdbe70c1532db042b64 AS web
WORKDIR /src/web
COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml ./
# The pnpm version pinned in package.json (packageManager), as in CI.
RUN npm install --global "$(node -p 'require("./package.json").packageManager')" \
    && pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm run build

# The server. Its queries are checked against the cache in server/.sqlx, so
# the build needs no database.
FROM rust:1-trixie@sha256:8a4e90c7946b74626488b1446f712c0fe0ed1a596cbd3d871684bc9c5433ae07 AS server
WORKDIR /src/server
ENV SQLX_OFFLINE=true
COPY server/ ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/server/target \
    cargo build --release --locked \
    && cp target/release/rueder /usr/local/bin/rueder
# The one directory the server writes to, for the stage below, which has no
# shell to make it with.
RUN mkdir -p /var/cache/rueder

# What runs: the program, the frontend and the C library, without a shell or a
# package manager, as a user that is not root.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:cc66d222f0a7ec996b78a99fe76a969bc80b539a5016ad866c14d1949bc6d0ab
LABEL org.opencontainers.image.title="rueder" \
      org.opencontainers.image.description="A self-hosted feed reader" \
      org.opencontainers.image.source="https://github.com/FionaPreroll/rueder"
COPY --from=server /usr/local/bin/rueder /usr/local/bin/rueder
COPY --from=web /src/web/build /usr/share/rueder/web
COPY --from=server --chown=nonroot:nonroot /var/cache/rueder /var/cache/rueder
# The cache of the image proxy and the key it makes for itself are all the
# server writes; everything else can be read-only. Secrets and DATABASE_URL
# come from the environment when the container starts, never from the image.
ENV LISTEN_ADDR=0.0.0.0:3000 \
    STATIC_DIR=/usr/share/rueder/web \
    IMAGE_CACHE_DIR=/var/cache/rueder
VOLUME /var/cache/rueder
EXPOSE 3000
USER nonroot
HEALTHCHECK --interval=30s --timeout=6s --start-period=60s --start-interval=2s --retries=3 \
    CMD ["rueder", "health"]
ENTRYPOINT ["rueder"]
CMD ["serve"]
