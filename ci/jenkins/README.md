# Local Jenkins (Docker) — VEdge core gate

A fully local CI you can run on your own machine (Docker), **off GitHub** — no
hosted-runner minutes. It runs the same core gate as `mise ci` + `mise audit`
(fmt-check · lint · test · wasm · audit) via the repo `Jenkinsfile`.

This is the "experience Jenkins" companion to the GitHub Actions gate
(`.github/workflows/ci.yml`). e2e is **not** run here — it needs a display + the
platform WebDriver; run `mise e2e` locally on Windows, or the opt-in GitHub e2e
job (`.github/workflows/e2e.yml`).

## What's here

- `../../Jenkinsfile` — declarative pipeline; one stage per gate step.
- `Dockerfile` — a Jenkins controller with the Rust toolchain, `wasm32` target,
  `leptosfmt`, `cargo-deny`, and `mise` baked in, so the built-in node has
  everything (no separate agents, no Docker-outside-of-Docker).
- `docker-compose.yml` — brings up the controller.

## Bring-up

```bash
# 1. Build + start the controller (first build compiles the toolchain image — slow once).
docker compose -f ci/jenkins/docker-compose.yml up -d --build

# 2. Grab the initial admin password.
docker exec vedge-jenkins cat /var/jenkins_home/secrets/initialAdminPassword
```

3. Open <http://localhost:8080>, paste the password, **Install suggested
   plugins**, and create the first admin user. (The suggested set includes the
   Pipeline + Git plugins this needs.)

4. **New Item → Pipeline** (name it e.g. `vedge-core-gate`):
   - **Pipeline → Definition: _Pipeline script from SCM_**
   - **SCM: Git**, Repository URL = this repo (a local path like
     `/var/jenkins_home/... ` won't see your working tree — use the Git remote,
     or mount the repo and point at `file:///repo`).
   - **Branch:** `*/develop` (or your feature branch).
   - **Script Path:** `Jenkinsfile`
   - Save → **Build Now**.

The pipeline runs `mise trust` then the five gate stages; a green run means the
core gate passes in a clean container.

## Notes

- **First build is slow** (compiling `leptosfmt` + `cargo-deny` + fetching the
  Rust toolchain into the image); subsequent builds reuse the image, and cargo
  artifacts persist in the `jenkins_home` volume, so incremental runs are quick.
- **Toolchain lives under `/opt/rust`**, deliberately *not* under
  `/var/jenkins_home` — that path is the volume mount and would shadow anything
  baked into the image.
- To wipe state: `docker compose -f ci/jenkins/docker-compose.yml down -v`.
- Building `vedge-tauri` on Linux needs the WebKitGTK dev libs (baked into the
  image). If you ever move the pipeline off this image, install
  `libwebkit2gtk-4.1-dev` + friends first.
