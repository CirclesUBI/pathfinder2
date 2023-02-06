FROM rust:latest AS build

WORKDIR /build
COPY . .

RUN cargo install --path .
RUN cargo build --release

FROM rust AS app

WORKDIR /app
COPY --from=build /build/target/release .
RUN chmod +x ./server

ENTRYPOINT ["./server"]
