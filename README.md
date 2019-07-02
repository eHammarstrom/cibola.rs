# Cibola

A json toy parser

### current speed, or a lack thereof

![benchmark-image](benches/current_bench.png)

```
parsing/CIBOLA::canada  time:   [20.725 ms 20.924 ms 21.120 ms]
                        thrpt:  [101.65 MiB/s 102.60 MiB/s 103.58 MiB/s]

parsing/CIBOLA::citm_catalog
                        time:   [14.171 ms 14.259 ms 14.354 ms]
                        thrpt:  [149.56 MiB/s 150.56 MiB/s 151.49 MiB/s]

parsing/serde_json::canada
                        time:   [15.307 ms 15.458 ms 15.655 ms]
                        thrpt:  [137.13 MiB/s 138.88 MiB/s 140.24 MiB/s]

parsing/serde_json::citm_catalog
                        time:   [9.1479 ms 9.3021 ms 9.4699 ms]
                        thrpt:  [226.69 MiB/s 230.78 MiB/s 234.67 MiB/s]

parsing/json-rust::canada
                        time:   [11.673 ms 11.735 ms 11.819 ms]
                        thrpt:  [181.63 MiB/s 182.94 MiB/s 183.91 MiB/s]

parsing/json-rust::citm_catalog
                        time:   [5.0979 ms 5.1819 ms 5.2747 ms]
                        thrpt:  [407.00 MiB/s 414.28 MiB/s 421.11 MiB/s]
```

