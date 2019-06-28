# Cibola

A json toy parser

### current speed, or lack thereof

```
parsing/CIBOLA::canada  time:   [23.727 ms 23.801 ms 23.875 ms]
                        thrpt:  [89.918 MiB/s 90.198 MiB/s 90.478 MiB/s]

parsing/serde_json::canada
                        time:   [14.525 ms 14.556 ms 14.596 ms]
                        thrpt:  [147.08 MiB/s 147.49 MiB/s 147.80 MiB/s]

parsing/json-rust::canada
                        time:   [10.804 ms 10.848 ms 10.896 ms]
                        thrpt:  [197.03 MiB/s 197.90 MiB/s 198.71 MiB/s]
```

