FROM rust:latest AS build

WORKDIR /build
COPY . .

RUN cargo install --path .
RUN cargo build --release

FROM rust AS app
LABEL org.opencontainers.image.source=https://github.com/circlesland/pathfinder2

WORKDIR /app
COPY --from=build /build/target/release .
RUN chmod +x ./server

CMD ["./server"]