####################################################################################################
## Builder
####################################################################################################
FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get update && apt-get install -y musl-tools musl-dev
RUN update-ca-certificates

# Create appuser
ENV USER=app
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --uid "${UID}" \
    "${USER}"


WORKDIR /app

COPY ./ .

RUN cd examples/containerized && cargo build --bin containerized-server --target x86_64-unknown-linux-musl

####################################################################################################
## Final image
####################################################################################################
FROM scratch

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group
COPY --from=builder /home /home

WORKDIR /app

# Copy our build
COPY --from=builder /app/examples/target/x86_64-unknown-linux-musl/debug/containerized-server ./

# Use an unprivileged user.
#USER app:app
CMD ["/app/containerized-server"]