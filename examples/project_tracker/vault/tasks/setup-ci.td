---
_type: Task
_label: "Set up CI pipeline"
status: "todo"
priority: "low"
project: fref("projects/mobile-app.td")
tags:
  - "ci"
  - "devops"
  - "github-actions"
  - "automated-testing"
  - "security-scanning"
  - "caching"
  - "matrix-build"
  - "docker"
  - "linting"
  - "artifact-publishing"
  - "notifications"
  - "infrastructure"
---

Configure a CI pipeline for the mobile app repo.
Should run linting, unit tests, and produce a build artifact on every pull request.

## Pipeline Stages

| Stage | Runs On | Trigger |
|-------|---------|---------|
| Lint | Ubuntu | Every push |
| Unit tests | Ubuntu | Every push |
| iOS build | macOS | Pull request |
| Android build | Ubuntu | Pull request |

## Steps

1. Check out repo and install dependencies
2. Run linter with zero-warning policy
3. Run unit test suite
4. Build iOS archive on macOS runner
5. Build APK on Ubuntu runner
6. Upload artifacts to storage

No assignee yet. To be picked up once the project moves out of planning.
