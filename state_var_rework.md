# Implementation Plan: `next/2` as a Constraint Relation

## Goal

Replace the `next(X)` term constructor with a `next/2` constraint relation, enabling `preserve` to be a stdlib rule rather than parser sugar.

## Design Summary

**Current:**
- State vars are uppercase (`Health`), stored as IR `Var` terms
- `next(Health)` is a term constructor, specially handled by compiler
- `preserve(X)` is parser sugar → `eq(next(X), X)`

**New:**
- State vars remain uppercase (`Health`), stored as IR `Var` terms (no change)
- `next(Health, V)` is a constraint relation posting to next-state store
- `preserve(X)` is a stdlib rule: `next(X, X) -------- preserve(X)`

The key insight: the `VarId` of a state variable **is** its identity. We don't need atoms as names — `next(Health, V)` identifies the state var by its `VarId`, resolved by walking the first argument.

---

## Design Rationale

### Why not atoms for state variable names?

We initially considered making state variables lowercase atoms (like `health`) so that `preserve(health)` would pass a name that could be looked up. However, this creates a problem:

If `health` is an atom, then `health = X` (structural unification) only succeeds when `X` unifies with the atom `health` itself — it doesn't give us the *value* associated with the state variable.

The current design works because state variables are **IR variables** that get unified with their values via facts like `eq(Health, 100)`. The variable `Health` is bound to `100` in the substitution.

### Why `next/2` works with variables

With `next(Var, Value)`:
1. The first argument resolves (via walking) to a state variable's `VarId`
2. The `VarId` **is** the identity — we don't need a separate name
3. The second argument is the value for the next state

### Why `preserve(X)` is just `next(X, X)`

When `preserve(Health)` is called:
1. `X = Health` (unifies with the state var)
2. Premise: `next(Health, Health)`
3. Engine walks first arg → gets `VarId` for Health
4. Engine walks second arg → gets current value of Health (e.g., `100`)
5. Records: next_state[VarId] = 100

No `current/2` relation needed — the current value is obtained by walking the variable through the substitution.

### Efficiency: fail-fast on conflicts

When posting a `next` constraint, we check for conflicts immediately:
- If no existing entry: add binding
- If existing entry: attempt unification
  - Same value or compatible variables: succeed
  - Conflicting values: fail immediately, prune branch

This is more efficient than letting conflicts produce 0 solutions at the end, because we avoid exploring branches that will ultimately fail.

---

## Implementation Steps

### 1. Parser (`src/ast/parser.rs`)

**1a.** Lines 402-413: Remove `preserve` desugaring in `parse_app` (delete the `if rel_name == "preserve"` block)

---

### 2. IR (`src/solver/ir.rs`)

**2a.** Lines 85-90: Add `Next` variant to `RelKind` enum

**2b.** Line 118: Remove `next_var_map` field from `Stage` struct

---

### 3. Compiler (`src/ast/compile.rs`)

**3a.** Remove `next_var_map` field (line 58) and `get_or_create_next_var` method (lines 151-163)

**3b.** Lines 185-189: Remove special `next(VarName)` handling in `lower_term_arg` — let it compile as regular `App` term

**3c.** ~Line 104: Register `next` as built-in relation with arity 2, kind `RelKind::Next`

**3d.** Lines 352-377: Update `lower_stage` to not capture/clear `next_var_map`

---

### 4. Engine (`src/solver/engine.rs`)

**4a.** Add `next_state: im::HashMap<VarId, TermId>` field to `State` struct (around line 386).

**4b.** Add method to `State`:
```rust
fn add_next_constraint(&self, var_id: VarId, value: TermId, subst: &Subst, terms: &Arena<Term>) -> Option<Self>
```
- Walk the value through subst first
- If no existing entry for var_id: add binding, return new state
- If existing entry: unify existing value with new value
  - Success: update with unified result
  - Failure: return None (conflict)

**4c.** Lines ~731-757: Handle `RelKind::Next` in `step_prop` `Prop::App` match:
- Walk `args[0]` through substitution
- Check it resolves to `Term::Var(var_id)` where var_id is a state variable
- Walk `args[1]` to get the value
- Call `state.add_next_constraint(var_id, value, ...)`
- If Some, push new state; if None, fail (don't push)

**4d.** Add accessor `pub fn next_state(&self) -> &HashMap<VarId, TermId>` to retrieve next-state map from a solved `State`

---

### 5. Frontend (`src/frontend.rs`)

**5a.** Lines 336-378 `build_transition_query`: Remove `next_var_map` from `TransitionQuery` struct — no longer needed

**5b.** Lines 380-458 `process_transition_result`:
- Extract next-state values from solution's `next_state` HashMap
- For each `(var_id, value_term_id)`, find state var name via reverse lookup in `state_var_term_ids`, then update

**5c.** `TransitionQuery` struct simplifies — just `goal` and `stage_name`

---

### 6. Stdlib (`src/stdlib.l`)

Add preserve rule:
```
Rule Preserve:
    next(X, X)
    ----------
    preserve(X)
```

---

### 7. Tests

**7a.** `src/tests/parser_tests.rs`:
- Remove `test_parse_preserve_*` tests (4 tests) — preserve is no longer parser sugar

**7b.** `src/tests/state_tests.rs`:
- Tests should continue to work with minimal changes
- Add test that `preserve(Health)` works via stdlib rule

**7c.** Add new engine tests:
- `next(Health, 50)` constrains next state correctly
- Conflicting `next` constraints (different values for same var) fail
- `next(Health, Health)` preserves current value

---

### 8. Documentation (`AGENTS.md`)

- Update `next()` documentation to show `next/2` relation semantics
- Update `preserve()` documentation (now stdlib rule, same user-facing behavior)
- Remove the `next/1` term constructor documentation

---

## Key Design Decisions

1. **State vars remain as uppercase variables**: No syntax change for users. The `VarId` is the identity.

2. **`next_state` in State**: Uses `im::HashMap<VarId, TermId>` for immutable updates during search. Keyed by `VarId` since that's the state variable's identity.

3. **Conflict handling**: Fail at constraint-posting time with unification check — more efficient than letting conflicts produce 0 solutions.

4. **`preserve(X)` is just `next(X, X)`**: Elegantly simple. When X is a state variable, this says "next value = current value".

---

## What Stays the Same

- State variable declaration syntax: `StateVar Health`
- State variable naming: uppercase (`Health`)
- User code referencing state vars: `Health = V`, `int_sub(Health, 10, NewHealth)`
- Fact structure: `eq(Health, 100)`

---

## What Changes

- `next(Health)` term constructor → `next(Health, V)` constraint relation
- `preserve(X)` parser sugar → stdlib rule
- Compiler no longer tracks `next_var_map` per stage
- Engine tracks `next_state` in `State` during search
- Frontend extracts next values from solution's `next_state` instead of stage's `next_var_map`

---

## No Breaking Changes

User-facing syntax remains identical. The only change is internal: how `next` and `preserve` are implemented.

---

## Theoretical Background

### Relational Logic and Constraint Relations

In relational logic (miniKanren-style), relations can be used in multiple modes:
- **Query mode**: find values that satisfy the relation
- **Constraint mode**: assert that certain relationships must hold

SMT relations like `int_add(X, Y, Z)` work as constraints — they don't compute, they constrain. The solver collects these constraints and solves them together.

`next/2` follows the same pattern: `next(Var, Value)` doesn't query anything, it **asserts** that the state variable's next value must be `Value`. The state transition solver collects all `next/2` assertions and uses them to build the next state.

### Why `preserve` couldn't be a rule before

The old `next(X)` was a **term constructor**, not a relation. It produced a fresh variable representing "the next value of X". This is a metalinguistic construct that only had meaning during compilation.

To write `preserve` as a rule, we needed `next` to be a **relation** that could be used in rule premises. The new `next/2` design makes this possible.

### State Transitions as Constraint Solving

A stage's state constraints define a relation between current state and next state. The solver:
1. Starts with current state values (from facts)
2. Processes constraints, collecting `next/2` assertions
3. Verifies exactly one solution (deterministic transition)
4. Extracts next values from the solution's `next_state` map
5. Updates facts for the new state

This is similar to how Constraint Handling Rules (CHR) or CLP handle state — you post constraints during execution that get solved together.
