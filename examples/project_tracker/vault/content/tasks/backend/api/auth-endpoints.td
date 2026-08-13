---
_type: Task
_label: "Implement auth endpoints"
title: "Implement auth endpoints"
status: "in_progress"
priority: "high"
project: fref("projects/mobile-app.td")
---

Build the authentication REST endpoints for the mobile app.

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | /auth/login | Email and password login |
| POST | /auth/register | New user registration |
| POST | /auth/refresh | Refresh access token |
| DELETE | /auth/logout | Invalidate session |
