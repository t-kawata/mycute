# Implementation: M-1-1 EvaluateCandidatesStep

## Changed Files

| File | Change |
|------|--------|
| `src/constants.rs` | Added `EVALUATION_THRESHOLD = 0.50` constant in new Evaluation Thresholds section |
| `src/error.rs` | Added `DarviumError::InvalidScore(f64)` variant |
| `src/types.rs` | Added `WorkflowGraphId`, `GraphPatch`, `CompositionPlan`, `SearchAbortReason`, `SearchOutcome` types; added `evaluate_candidates()` and `apply_self_conf_discount()` functions; added 18 tests (T1-T5 + OTS-1/OTS-2); fixed OscillationDetector clippy warning |
| `src/lib.rs` | Updated re-exports to include `SearchOutcome`, `evaluate_candidates`, `apply_self_conf_discount` |

## Implementation Details

- `SearchOutcome` enum: 6 variants (ReuseExisting, PatchExisting, ComposeExisting, GenerateNew, AbortSearch, NeedsHumanReview), manual PartialEq since petgraph::Graph doesn't impl PartialEq
- `evaluate_candidates(best_score: f64) -> Result<SearchOutcome, DarviumError>`: validates [0.0, 1.0] range, score >= 0.50 → ReuseExisting, else → PatchExisting
- `apply_self_conf_discount(raw_score: f64) -> f64`: raw * 0.85 clamped to [0.0, 1.0]
- OTS tests use Box-Muller transform for Gaussian noise (rand 0.9 compat)

## Verification

- `cargo test`: 212 passed
- `cargo clippy -- -D warnings`: clean
- `cargo fmt`: clean
- Boy Scout: Fixed OscillationDetector::default() clippy warning by implementing Default trait

## Test Coverage (18 new tests)

- T1: 5 tests (normal threshold evaluation)
- T2: 5 tests (invalid score rejection)
- T3: 2 tests (determinism)
- T4: 4 tests (self-confidence discount)
- T5: 2 tests (enum traits + exhaustive matching)
- OTS-1: 1 test (decision boundary distribution with noise sweep)
- OTS-2: 1 test (scaling law verification)
