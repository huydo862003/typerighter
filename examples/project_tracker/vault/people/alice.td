---
_type: Person
_label: "Alice Chen (Developer)"
name: "Alice Chen"
role: "developer"
active: true
tags: ["rust", "backend", "auth"]
email: "alice@example.com"
tasks:
  - fref("tasks/implement-auth.td")
  - fref("tasks/design-mockups.td")
  - fref("tasks/write-tests.td")
  - fref("tasks/setup-ci.td")
---

Alice is a **backend developer** with a focus on *authentication systems* and developer tooling.
She joined the team in 2023 and has been ***leading*** the website redesign effort.

## Skills

| Area | Proficiency |
|------|-------------|
| Rust | Expert |
| TypeScript | Proficient |
| PostgreSQL | Proficient |
| Docker | Familiar |

## Responsibilities

- Lead backend development on the website redesign
- Review pull requests for the auth subsystem
- Mentor junior developers on the team

::: details Weekly schedule
| Day | Focus |
|-----|-------|
| Monday | Sprint planning, code review |
| Tuesday | Deep work on auth implementation |
| Wednesday | Pairing sessions with junior devs |
| Thursday | Deep work, PR reviews |
| Friday | Documentation, tech debt |
:::

## Tools

Alice uses a custom shell function to check CI status:

```bash
ci-status() {
  gh run list --repo "$1" --limit 5 --json status,conclusion,name \
    | jq '.[] | "\(.name): \(.status) (\(.conclusion // "pending"))"'
}
```

## Inline References

Currently working on: ${fref("tasks/implement-auth.td")}
