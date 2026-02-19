# Phase 02 Memo for Phase 03

This document records ideas for improvements and tasks deferred to Phase 03, identified during the Phase 02 implementation.

## Potential Improvements

- [ ] **Refine Error Handling**: Currently, some errors are logged and fatal. We should implement more graceful error handling and propagation.
- [ ] **Configuration Management**: Move hardcoded paths (like `cognee_v2.db`) and settings to a proper configuration file or environment variable system (e.g., using `viper`).
- [ ] **DuckDB Connection Management**: Investigate better ways to manage DuckDB connections to avoid lock issues during development/testing (e.g., using a connection pool or a singleton service).
- [ ] **CLI UX**: Improve the CLI output to be more user-friendly and informative.

## Phase 2C2 Insights

- [ ] **Unanswerable Question Detection**: The LLM sometimes hallucinates answers for questions not covered in the knowledge base (e.g., "フランスの首都" → "パリ"). Consider implementing:
  - Confidence thresholds based on vector similarity scores
  - Explicit "no relevant context" detection before LLM call
  - Few-shot prompting with examples of unanswerable responses

- [ ] **Graph Traversal Enhancement**: Current search uses only vector similarity on chunks. Phase 3 should implement:
  - Multi-hop graph traversal from matching nodes
  - Triplet-based context enrichment for RAG
  - Hybrid search combining vector + graph results

- [ ] **Embedding Model Selection**: Consider allowing configurable embedding models (e.g., `text-embedding-3-small` vs `text-embedding-3-large`) for different accuracy/cost tradeoffs.

- [ ] **Benchmark Enhancements**:
  - Add `--verbose` flag for detailed per-question output
  - Export results to JSON/CSV for analysis
  - Support for multiple QA files in batch mode

