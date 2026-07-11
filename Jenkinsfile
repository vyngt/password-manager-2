// VEdge local CI — the core gate (fmt · lint · test · wasm · audit), the same
// `mise` tasks as the GitHub Actions gate and local `mise ci`. Runs off GitHub
// entirely in a Dockerised Jenkins (see `ci/jenkins/`), so it never consumes
// GitHub-hosted runner minutes. e2e stays out (it needs a display + WebDriver —
// run `mise e2e` locally on Windows, or the opt-in GitHub e2e job).
//
// The controller image (ci/jenkins/Dockerfile) bakes in the Rust toolchain,
// wasm target, leptosfmt, cargo-deny and mise, so the built-in node has
// everything the tasks need.
pipeline {
    agent any

    options {
        timestamps()
        disableConcurrentBuilds()
        timeout(time: 45, unit: 'MINUTES')
    }

    stages {
        stage('trust') {
            // mise refuses to run an untrusted config; trust the checked-out repo.
            steps { sh 'mise trust --yes' }
        }
        stage('fmt-check') {
            steps { sh 'mise run fmt-check' }
        }
        stage('lint') {
            steps { sh 'mise run lint' }
        }
        stage('test') {
            steps { sh 'mise run test' }
        }
        stage('wasm') {
            steps { sh 'mise run wasm' }
        }
        stage('audit') {
            steps { sh 'mise run audit' }
        }
    }
}
