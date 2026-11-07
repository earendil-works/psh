# The recording container for the goldens (TESTING.md "The container"): Debian stable
# with its GNU userland and GNU bash 5.3 built from the release tarball plus the
# official patches through 5.3.9, each pinned by sha256. Built and run by
# scripts/record-goldens. Nobody reads bash's source; it is only compiled (D007).
#
# 5.3.9 matches the Homebrew bash 5.3 used for quick probes on macOS, so a probe and
# the goldens differ only by platform, not by version. Bumping the patch level means
# adding the patches below and re-recording every golden in one commit.

FROM debian:stable@sha256:3f0347f4b50e7a54a91d1beaf6ce6fa5aec392347da6a9184b5bfe4e1b814538 AS build
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl gcc make libc6-dev patch \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /build
RUN set -eu; \
    base=https://ftp.gnu.org/gnu/bash; \
    curl -fsSLO "$base/bash-5.3.tar.gz"; \
    for i in 001 002 003 004 005 006 007 008 009; do \
      curl -fsSLO "$base/bash-5.3-patches/bash53-$i"; \
    done; \
    printf '%s\n' \
      '0d5cd86965f869a26cf64f4b71be7b96f90a3ba8b3d74e27e8e9d9d5550f31ba  bash-5.3.tar.gz' \
      '1f608434364af86b9b45c8b0ea3fb3b165fb830d27697e6cdfc7ac17dee3287f  bash53-001' \
      'e385548a00130765ec7938a56fbdca52447ab41fabc95a25f19ade527e282001  bash53-002' \
      'f245d9c7dc3f5a20d84b53d249334747940936f09dc97e1dcb89fc3ab37d60ed  bash53-003' \
      '9591d245045529f32f0812f94180b9d9ce9023f5a765c039b852e5dfc99747d0  bash53-004' \
      'cca1ef52dbbf433bc98e33269b64b2c814028efe2538be1e2c9a377da90bc99d  bash53-005' \
      '29119addefed8eff91ae37fd51822c31780ee30d4a28376e96002706c995ff10  bash53-006' \
      'c0976bbfffa1453c7cfdd62058f206a318568ff2d690f5d4fa048793fa3eb299  bash53-007' \
      '097cd723cbfb8907674ac32214063a3fd85282657ec5b4e544d2c0f719653fb4  bash53-008' \
      'eee30fe78a4b0cb2fe20e010e00308899cfc613e0774ebb3c8557a1552f24f8c  bash53-009' \
      | sha256sum -c -; \
    tar xzf bash-5.3.tar.gz; \
    cd bash-5.3; \
    for i in 001 002 003 004 005 006 007 008 009; do patch -p0 -s < "../bash53-$i"; done; \
    ./configure --prefix=/usr/local --quiet; \
    make -s -j"$(nproc)"; \
    make -s install

FROM debian:stable@sha256:3f0347f4b50e7a54a91d1beaf6ce6fa5aec392347da6a9184b5bfe4e1b814538
# The GNU userland the cases may call: coreutils, grep, sed, findutils, diffutils are in
# the base image; gawk replaces mawk as awk; procps provides ps; locales provides
# en_US.UTF-8, which a few cases switch to and typical agent machines have. git, ripgrep
# and jq for the agent layer (0031, D1310): agents run them in a quarter of their
# commands. Pinned to the versions of this Debian image; a rebuild after Debian drops one
# of them needs snapshot.debian.org or a new pin and a re-recording.
RUN apt-get update \
 && apt-get install -y --no-install-recommends gawk procps locales \
      git=1:2.47.3-0+deb13u1 ripgrep=14.1.1-1+b4 jq=1.7.1-6+deb13u4 \
 && rm -rf /var/lib/apt/lists/* \
 && sed -i 's/^# *en_US.UTF-8/en_US.UTF-8/' /etc/locale.gen && locale-gen
COPY --from=build /usr/local/bin/bash /usr/local/bin/bash
# Cases run as an ordinary user (root passes every permission test), with a fixed
# name, home, hostname (set by `docker run --hostname`) and umask.
RUN useradd --create-home --uid 1000 spec
USER spec
WORKDIR /home/spec
ENV LC_ALL=C.UTF-8 TZ=UTC HOME=/home/spec USER=spec
ENTRYPOINT ["/bin/sh", "-c", "umask 022 && exec \"$@\"", "--"]
