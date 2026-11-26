FROM rust:1-slim-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock askama.toml ./
COPY src ./src
COPY migrations ./migrations
COPY static ./static
COPY tests ./tests
RUN cargo build --release --bin scholarly

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/scholarly /usr/local/bin/scholarly
COPY --from=build /app/static ./static
ENV BIND=0.0.0.0
EXPOSE 8080
CMD ["scholarly", "serve"]
