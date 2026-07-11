# Jenkins controller with the VEdge core-gate toolchain baked in, so pipeline
# builds run on the built-in node with no extra agent wiring (no docker socket,
# no Docker-outside-of-Docker). Heavier image, but the simplest thing that
# reliably runs `mise ci` + `mise audit` locally.
#
# Build + run via ci/jenkins/docker-compose.yml. See ci/jenkins/README.md.
FROM jenkins/jenkins:lts-jdk17

# --- system deps (root) ---------------------------------------------------------
# Tauri (vedge-tauri) links WebKitGTK on Linux, so `cargo test` — which compiles
# the shell crate — needs these to build. Plus the usual build essentials.
USER root
RUN apt-get update && apt-get install -y --no-install-recommends \
        curl ca-certificates build-essential pkg-config git \
        libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
        librsvg2-dev libsoup-3.0-dev \
    && rm -rf /var/lib/apt/lists/* \
    # Rust + mise live under /opt (NOT /var/jenkins_home — that path is a volume
    # at runtime and would shadow anything baked into it).
    && mkdir -p /opt/rust && chown -R jenkins:jenkins /opt/rust

# --- Rust toolchain + gate tooling (jenkins user) -------------------------------
USER jenkins
ENV RUSTUP_HOME=/opt/rust/rustup \
    CARGO_HOME=/opt/rust/cargo \
    MISE_DATA_DIR=/opt/rust/mise \
    PATH=/opt/rust/cargo/bin:${PATH}

# Pinned to match the project's dev toolchain so CI clippy == local clippy (a
# newer stable's stricter lints would diverge). Bump in lockstep with local.
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.96.0 --profile default \
    && rustup target add wasm32-unknown-unknown \
    && rustup component add rustfmt clippy \
    && cargo install leptosfmt cargo-deny --locked \
    # Install mise onto the PATH (again, not under the jenkins_home volume).
    && curl -fsSL https://mise.run | MISE_INSTALL_PATH=/opt/rust/cargo/bin/mise sh
