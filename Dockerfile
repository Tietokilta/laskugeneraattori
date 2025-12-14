# https://hub.docker.com/_/rust
FROM rust:alpine as builder
RUN apk --no-cache add musl-dev
# Create a new empty shell project
WORKDIR /app

# Copy over the Cargo.toml files to the shell project
COPY Cargo.toml Cargo.lock build.rs ./
# Build and cache the dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs
RUN cargo fetch
RUN cargo build --release
RUN rm src/main.rs src/lib.rs

# Copy the actual code files and build the application
COPY ./src ./src
COPY ./templates ./templates
# Update the file date so Cargo rebuilds it
ARG GIT_COMMIT_SHA=development
ENV GIT_COMMIT_SHA=$GIT_COMMIT_SHA
RUN touch src/main.rs
RUN cargo build --release

FROM alpine as runtime
ENV BIND_ADDR 0.0.0.0
WORKDIR /app
COPY --from=builder /app/target/release/laskugeneraattori app
EXPOSE 3000
CMD ["/app/app"]
