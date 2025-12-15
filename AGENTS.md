# Langame AST Documentation

This document describes the Abstract Syntax Tree (AST) structure for the Langame relational language parser.

## Overview

The Langame AST is a hierarchical structure consisting of four main levels:

```
Module
  └── Stage(s)
        └── Rule(s)
              ├── Premise (Term)
              └── Conclusion (Term)
```

## AST Node Types

### Module

Represents a complete Langame file.

**Structure:**
```rust
pub struct Module<'a> {
    pub span: Span<'a>,
    pub stages: Vec<Stage<'a>>
}
```

**Fields:**
- `span`: Source location information
- `stages`: Zero or more stages

**Grammar:**
```
Module = <stage>*
```

**Example:**
```
Begin Stage Arithmetic:
  ...
End Stage Arithmetic

Begin Stage Logic:
  ...
End Stage Logic
```

### Stage

Represents a named collection of rules.

**Structure:**
```rust
pub struct Stage<'a> {
    pub span: Span<'a>,
    pub name: &'a str,
    pub rules: Vec<Rule<'a>>,
}
```

**Fields:**
- `span`: Source location information
- `name`: Stage identifier
- `rules`: Zero or more rules within this stage

**Grammar:**
```
Begin Stage <StageName>:
<rule>*
End Stage <StageName>
```

**Example:**
```
Begin Stage Arithmetic:
Rule Add:
    add(X, Y)
    ---------
    sum(X, Y)
End Stage Arithmetic
```

### Rule

Represents an inference rule with a premise and conclusion.

**Structure:**
```rust
pub struct Rule<'a> {
    pub span: Span<'a>,
    pub name: &'a str,
    pub premise: Term<'a>,
    pub conclusion: Term<'a>,
}
```

**Fields:**
- `span`: Source location information
- `name`: Rule identifier
- `premise`: The condition or input term
- `conclusion`: The result or output term

**Grammar:**
```
Rule <name>:
    term     # premise
    --------
    term     # conclusion
```

**Example:**
```
Rule AddCommutative:
    add(X, Y)
    ---------
    add(Y, X)
```

### Term

Represents an expression in the language.

**Structure:**
```rust
pub struct Term<'a> {
    pub span: Span<'a>,
    pub contents: TermContents<'a>,
}

pub enum TermContents<'a> {
    App { rel: Rel<'a>, args: Vec<Term<'a>> },
    Atom { text: &'a str },
    Var { name: &'a str },
    Int { val: i32 },
    Float { val: f32 },
}
```

**Variants:**

1. **Application (App)**: Function/relation application
   - `rel`: The relation being applied (SMT or user-defined)
   - `args`: Arguments to the relation
   - Example: `add(X, Y)`, `mul(3, 4)`

2. **Atom**: Lowercase identifier
   - `text`: The atom text
   - Example: `foo`, `bar`, `int`

3. **Variable (Var)**: Uppercase identifier
   - `name`: The variable name
   - Example: `X`, `Y`, `Result`

4. **Integer (Int)**: Numeric integer
   - `val`: The integer value
   - Example: `42`, `-5`

5. **Float**: Numeric floating-point
   - `val`: The float value
   - Example: `3.14`, `-2.5`

**Grammar:**
```
term = <atom>                    # lowercase identifier
     | <int>                     # integer literal
     | <float>                   # float literal
     | <var>                     # uppercase identifier
     | <relation>(<term>,*)      # application
```

**Examples:**
```
X                    # Variable
42                   # Integer
3.14                 # Float
foo                  # Atom
add(X, Y)            # Application with 2 args
mul(add(1, 2), 3)    # Nested application
typeof(42, int)      # Mixed types
```

### Relation

Represents the relation/function in an application.

**Structure:**
```rust
pub enum Rel<'a> {
    SMTRel { name: &'a str },
    UserRel { name: &'a str },
}
```

**Variants:**
1. **SMTRel**: Built-in SMT solver relation
2. **UserRel**: User-defined relation

Currently, the parser treats all relations as `UserRel`.

## Display Format

The AST implements `Display` for pretty-printing:

### Module Display
```
Begin Stage <stage1>:
<rules>
End Stage <stage1>

Begin Stage <stage2>:
<rules>
End Stage <stage2>
```

### Stage Display
```
Begin Stage <name>:
Rule <rule1>:
    <premise>
    <dashes>
    <conclusion>
Rule <rule2>:
    <premise>
    <dashes>
    <conclusion>
End Stage <name>
```

### Rule Display
```
Rule <name>:
    <premise>
    <dashes>
    <conclusion>
```

The divider length matches the longer of the premise or conclusion.

### Term Display
- **Application**: `relation(arg1, arg2, ...)`
- **Atom**: `atom`
- **Variable**: `Variable`
- **Integer**: `42`
- **Float**: `3.14`

## Parser Functions

The parser provides three main entry points:

1. **`parse_module`**: Parses a complete module (entire file)
2. **`parse_stage`**: Parses a single stage
3. **`parse_rule`**: Parses a single rule
4. **`parse_term`**: Parses a single term

All functions return `IResult<Span, T>` where `T` is the corresponding AST node type.

## Example Complete File

```
Begin Stage TypeSystem:
Rule IntegerType:
    typeof(42, int)
    ---------------
    valid(42)

Rule AdditionType:
    typeof(add(X, Y), int)
    ----------------------
    valid(add(X, Y))
End Stage TypeSystem

Begin Stage Arithmetic:
Rule AddCommutative:
    add(X, Y)
    ---------
    add(Y, X)

Rule Distributive:
    mul(add(X, Y), Z)
    -----------------
    add(mul(X, Z), mul(Y, Z))
End Stage Arithmetic
```

This would parse into:
- A `Module` with 2 stages
- Stage 1 ("TypeSystem") with 2 rules
- Stage 2 ("Arithmetic") with 2 rules
- Each rule containing premise and conclusion terms with various structures (applications, variables, atoms, integers)

## Notes

- Identifiers starting with uppercase letters are parsed as **variables**
- Identifiers starting with lowercase letters are parsed as **atoms**
- The parser uses `nom` for parsing and `nom_locate` for span tracking
- All AST nodes carry span information for error reporting and source mapping
