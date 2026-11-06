FROM rust:1-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends bash dash mksh zsh busybox python3 unzip curl ca-certificates coreutils procps && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://go.dev/dl/go1.25.4.linux-arm64.tar.gz | tar -C /usr/local -xz
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH=/usr/local/go/bin:/root/.bun/bin:$PATH
