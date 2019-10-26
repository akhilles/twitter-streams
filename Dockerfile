FROM rustlang/rust:nightly

WORKDIR /app
ADD . .

RUN cargo build --release
ENTRYPOINT [ "./target/release/twitter-streams" ]
