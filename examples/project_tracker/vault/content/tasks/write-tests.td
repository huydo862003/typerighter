---
_type: Task
title: "Write integration tests for auth"
status: "todo"
priority: "medium"
blocked: false
project: fref("projects/website-redesign.td")
assignee: fref("people/alice.td")
---

Depends on the auth implementation being merged first.
Tests should cover login success, login failure, session expiry, and password reset flow.

::: info Prerequisites
These tests require a running PostgreSQL database and Redis instance.
See the setup instructions below.
:::

## Test Cases

| Scenario | Expected Result |
|----------|----------------|
| Valid credentials | 200 with access token |
| Wrong password | 401 |
| Expired session | 401, prompt re-login |
| Password reset request | 200, email sent |
| Rate limit exceeded | 429 |

## Setup

```bash
docker compose up -d postgres
export DATABASE_URL="postgres://test:test@localhost:5432/auth_test"
cargo test --test auth_integration
```

The test harness automatically runs migrations on startup:

```rust{1,4-5}
#[tokio::test]
async fn test_login_success() {
    let app = TestApp::spawn().await;
    let response = app.post_login("alice@example.com", "correct-password").await;
    assert_eq!(response.status(), 200);
    assert!(response.json::<LoginResponse>().access_token.len() > 0);
}
```

## Checklist

- [ ] Set up test database fixtures
- [ ] Write happy path tests
- [ ] Write failure and edge case tests
- [ ] Assert token expiry behavior
- [x] Choose test framework

::: warning No flaky tests
Flaky tests must be fixed before merge. Do not use `#[ignore]` to skip them.
:::

::: tip Running locally
Use `cargo watch -x 'test --test auth_integration'` for fast feedback during development.
:::

## Coverage Target

::: note
Expected coverage: $\frac{\text{covered lines}}{\text{total lines}} \geq 0.80$
:::
