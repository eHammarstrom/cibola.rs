# Cibola

A json toy parser

### current speed, or lack thereof

```
parsing/CIBOLA::canada  time:   [23.176 ms 23.210 ms 23.244 ms]
                        thrpt:  [92.358 MiB/s 92.492 MiB/s 92.631 MiB/s]

parsing/serde_json::canada
                        time:   [14.542 ms 14.585 ms 14.622 ms]
                        thrpt:  [146.82 MiB/s 147.19 MiB/s 147.63 MiB/s]

parsing/json-rust::canada
                        time:   [10.819 ms 10.856 ms 10.915 ms]
                        thrpt:  [196.67 MiB/s 197.75 MiB/s 198.42 MiB/s]
```

