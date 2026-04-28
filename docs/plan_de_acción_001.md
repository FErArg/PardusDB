# Plan: Add `AS` alias for plain columns + refine GROUP BY

## Current State

**`AS` alias:** Only supported for aggregate functions (`COUNT(*) AS total`). The `SelectColumn::Aggregate` variant has `alias: Option<String>`. Plain columns (`SelectColumn::Column(String)`) have **no** alias field -- `SELECT title AS t FROM docs` fails.

**`GROUP BY`:** Fully implemented and working. Parsing at `parser.rs:880-902`, execution at `database.rs:461-643` with hash aggregation, HAVING, ORDER BY, LIMIT, OFFSET.

## Goal

1. Allow `SELECT col AS alias FROM table`
2. Resolve aliases in `ORDER BY`, `GROUP BY`, and `HAVING` clauses
3. Respect SQL evaluation order: `FROM -> WHERE -> GROUP BY -> HAVING -> SELECT -> ORDER BY`. Aliases are NOT available in WHERE or GROUP BY (only in HAVING and ORDER BY).

## Changes Required

### File: `src/parser.rs`

**A. Add alias field to `SelectColumn::Column`** (line 85)

```
pub enum SelectColumn {
    All,
    Column(String, Option<String>),  // (name, optional alias)
    Aggregate { func: AggregateFunc, column: String, alias: Option<String> },
}
```

**B. Update constructors** (lines 353, 372, 403)

All push `SelectColumn::Column(name)` -- change to `SelectColumn::Column(name, None)`.

**C. Add `AS` parsing after regular column identifiers** (around line 407)

After `select_columns.push(SelectColumn::Column(col, None))` at line 403, add alias check:

```
self.skip_whitespace();
if self.peek_keyword_upper() == "AS" {
    // pop last, re-push with alias
}
```

**D. Delete `parse_select_column()`** (lines 558-601)

It's dead code (compiler warns `#[warn(dead_code)]`). It handles alias for aggregates but returns `Column` without alias. Remove it entirely.

### File: `src/database.rs`

**E. Add alias resolution in `select()`** (around line 311)

Build an `alias_map: HashMap<String, String>` from the `SelectColumn` list before processing.

Resolve `order_by.column` through the alias map before calling `table.select()`.

**F. Update `execute_group_by()`** (lines 461-643)

Resolve GROUP BY columns through the alias map. Thread the alias map through so column names in result formatting use aliases for display.

**G. Update all `SelectColumn::Column(name)` destructures** (8 sites)

Each becomes `SelectColumn::Column(name, _alias)` -- use `alias` for display names, use `name` for internal lookups.

Sites:
- `database.rs:353` -- `SelectColumn::Column(name)` -> `SelectColumn::Column(name, _)`
- `database.rs:445` -- same
- `database.rs:499` -- same, use alias for display name: `alias.clone().unwrap_or(name)`
- `database.rs:519` -- same
- `concurrent.rs:573` -- same
- `tests/sql_parser_test.rs:112,113` -- `SelectColumn::Column(ref s, _)` with guard on `s`

**H. `matches_having_condition()`** (line 664)

The HAVING clause references display names (aliases or original). Check both `col_names` (which include aliased display names) and the raw column names.

## Potential Problems

| # | Problem | Severity | Mitigation |
|---|---------|----------|------------|
| 1 | 8 pattern-match sites break across 4 files | Low | Caught by compiler, trivial to update |
| 2 | `table.select()` receives unresolved aliases in `order_by` | Medium | Resolve alias BEFORE calling `table.select()`, not inside it. Add pre-processing step that converts order_by column via alias map. |
| 3 | `GROUP BY alias` needs resolution but SQL spec says GROUP BY doesn't see SELECT aliases | Medium | Accept SQL standard: GROUP BY references original column names, not aliases. Only HAVING and ORDER BY get alias resolution. Document this. |
| 4 | No alias map data structure exists today | Low | Simple `HashMap<String, String>` built once from SelectColumn list |
| 5 | `parse_select_column()` is dead code | Low | Delete it instead of updating it |
| 6 | No existing tests for alias scenarios | Low | Add explicit tests for: `ORDER BY alias`, `HAVING alias`, `plain AS`, `aggregate AS` regression |
| 7 | Public API breakage for external crate users | Low | Bump minor version. No known external users. Compiler catches all. |

## Downstream Impact Summary

| File | Changes |
|------|---------|
| `src/parser.rs` | ~15 lines: enum variant change + AS parsing + delete dead function |
| `src/database.rs` | ~30 lines: match arm updates + alias map + pre-resolution in select() |
| `src/concurrent.rs` | ~2 lines: match arm update |
| `tests/sql_parser_test.rs` | ~15 lines: update 2 existing matches + new alias tests |
| `tests/database_test.rs` | ~20 lines: integration tests for alias workflows |

## Execution Order

1. Edit `src/parser.rs` -- enum + constructors + AS parsing + delete dead function
2. Edit `src/database.rs` -- match arms + alias map + resolution
3. Edit `src/concurrent.rs` -- match arms
4. Edit `tests/sql_parser_test.rs` -- parser tests
5. Verify: `cargo test --lib graph && cargo test --lib parser && cargo test --test database_test && cargo test --test sql_parser_test`
6. Commit
