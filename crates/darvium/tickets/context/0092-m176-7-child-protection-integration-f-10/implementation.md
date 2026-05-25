# Implementation: M1.76-7 Child protection integration (F-10)

## Changed files

| File | Change | Description |
|---|---|---|
| src/constants.rs | Add 3 constants | CHILD_PROTECT_ETA1=0.50, CHILD_PROTECT_ETA2=0.30, CHILD_PROTECT_ETA3=0.20 (Calibration Candidates) |
| src/reciprocity.rs | Add function | `pub fn compute_child_protection(is_child, help_received, growth_improvement) -> f32` (F-10) |
| src/reciprocity.rs | Add 7 tests | TC-1~TC-7 covering zero-input, min-eta1, monotonicity x2, Grace Period independence, instrumentation response surface, eta sensitivity |

## Verification
- `cargo test`: 956 tests, 0 failed
- `cargo clippy`: 0 warnings
- All observational tests PASS with structured CSV output

## Public API
`darvium::reciprocity::compute_child_protection(is_child: bool, help_received: f32, growth_improvement: f32) -> f32`
