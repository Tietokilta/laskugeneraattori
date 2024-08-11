# https://hub.docker.com/_/rust
FROM rust:alpine as builder
RUN apk --no-cache add musl-dev
# Create a new empty shell project
WORKDIR /app

# Copy over the Cargo.toml files to the shell project
COPY Cargo.toml Cargo.lock ./
# Build and cache the dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch
RUN cargo build --release
RUN rm src/main.rs

# Copy the actual code files and build the application
COPY ./src ./src
COPY ./templates ./templates
# Update the file date so Cargo rebuilds it
RUN touch src/main.rs
RUN cargo build --release

FROM alpine as runtime
WORKDIR /app
COPY --from=builder /app/target/release/laskugeneraattori app
EXPOSE 5237
CMD ["/app/app"]
