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
# The directory of the tokens, and bash and env with their library, for the
# stage below, which has neither. Each comes with its package's entry, so that
# the scan of the image knows their versions.
RUN mkdir -p /etc/rssifier /out/var/lib/dpkg/status.d \
    && cp -a --parents /usr/bin/bash /usr/bin/env /usr/lib/*-linux-gnu/libtinfo.so.6* /out \
    && for package in bash coreutils libtinfo6; do \
        dpkg-query --status "$package" >"/out/var/lib/dpkg/status.d/$package"; \
    done

# The program, the C library, bash and env to look around in a running
# container, as a user that is not root; no other tools, no package manager.
# The tokens come in as a file the operator mounts (RSSIFIER_TOKENS), and
# `rssifier token` runs with `docker run` on the same file.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:cc66d222f0a7ec996b78a99fe76a969bc80b539a5016ad866c14d1949bc6d0ab
LABEL org.opencontainers.image.title="rssifier" \
      org.opencontainers.image.description="Makes feeds of web pages that have none" \
      org.opencontainers.image.source="https://github.com/FionaPreroll/rueder"
COPY --from=build /out/ /
COPY --from=build /usr/local/bin/rssifier /usr/local/bin/rssifier
COPY --from=build --chown=nonroot:nonroot /etc/rssifier /etc/rssifier
ENV LISTEN_ADDR=0.0.0.0:3100 \
    RSSIFIER_TOKENS=/etc/rssifier/tokens.json
VOLUME /etc/rssifier
EXPOSE 3100
USER nonroot
HEALTHCHECK --interval=30s --timeout=6s --start-period=30s --start-interval=2s --retries=3 \
    CMD ["rssifier", "health"]
ENTRYPOINT ["rssifier"]
CMD ["serve"]
