# Roundtrip harness

Cross-binary integration tests validating Rust↔Go SQLite compatibility.

## Activation

```bash
export RONDO_GO=/path/to/rondo-go-binary
cargo test -p rondo-tui --test roundtrip_smoke -- --include-ignored
```

Without `RONDO_GO`, the ignored tests stay ignored. The skeleton smoke
test (`harness_seed_smoke`) always runs to ensure helpers compile and
the fixture loads correctly.

## Test gating

| Test | Unlocks with |
|---|---|
| `harness_seed_smoke` | always |
| `rust_creates_go_reads` | M1.3 mutations API |
| `go_writes_rust_reads` | M1.3 mutations API |
| `recurrence_no_duplicates` | M4 recurrence engine |
| `backup_files_ignored_by_go` | M1.1 backup support |

This skeleton exists so Batch 7's GATE can be flipped the day those
milestones complete, without scrambling for harness code.
