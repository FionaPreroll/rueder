# RSSifier as one image, next to rueder. Built from the root of the
# repository, as it needs ../hardened-http:
#
#   docker build -f rssifier/Dockerfile -t rssifier .
#
# Base images as in ../Dockerfile, by tag and digest.

FROM rust:1-trixie@sha256:8a4e90c7946b74626488b1446f712c0fe0ed1a596cbd3d871684bc9c5433ae07 AS build
WORKDIR /src/rssifier
COPY hardened-http/ /src/hardened-http/
COPY rssifier/ ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/rssifier/target \
    cargo build --release --locked \
    && cp target/release/rssifier /usr/local/bin/rssifier
# The directory of the tokens, for the stage below, which has no shell.
RUN mkdir -p /etc/rssifier

# The program and the C library, without a shell, as a user that is not root.
# The tokens come in as a file the operator mounts (RSSIFIER_TOKENS), and
# `rssifier token` runs with `docker run` on the same file.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:cc66d222f0a7ec996b78a99fe76a969bc80b539a5016ad866c14d1949bc6d0ab
LABEL org.opencontainers.image.title="rssifier" \
      org.opencontainers.image.description="Makes feeds of web pages that have none" \
      org.opencontainers.image.source="https://github.com/FionaPreroll/rueder"
COPY --from=build /usr/local/bin/rssifier /usr/local/bin/rssifier
COPY --from=build --chown=nonroot:nonroot /etc/rssifier /etc/rssifier
ENV LISTEN_ADDR=0.0.0.0:3100 \
    RSSIFIER_TOKENS=/etc/rssifier/tokens.json
VOLUME /etc/rssifier
EXPOSE 3100
USER nonroot
ENTRYPOINT ["rssifier"]
CMD ["serve"]
