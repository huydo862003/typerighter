---
_type: Card
_label: 'What''s the difference between X and Y?'
title: 'It''s a test'
---

This file uses YAML-standard `''` escaping inside single-quoted strings.
Typerighter uses `\'` instead, so this should produce a diagnostic error,
not silently truncate the value to "What" / "It".
