# Roadmap

## Phase 1 — Core Signal API ✅

- [x] Basic Signal/Callable types (#1)
- [x] Deduplicated connections (#2)
- [x] is_empty() method (#3)

## Phase 2 — Emitting ✅

- [x] Invoke callbacks with a value (#4)
- [x] Test emit order matches insertion order (#5)

## Phase 3 — Lifecycle ✅

- [x] disconnect() method (#6)
- [ ] Weak connections / auto-disconnect on drop (#7) *(skipped — Signal owns Callables)*

## Phase 4 — Advanced

- [x] Multi-thread support (Arc + Mutex/ArcSwap) (#8)
- [ ] Signal chaining / composition (#9)
- [ ] Performance benchmarks (#10)
