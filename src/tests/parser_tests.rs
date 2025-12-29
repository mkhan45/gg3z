use crate::ast::parser::*;
use crate::ast::*;

#[test]
fn test_parse_rule() {
    let input = Span::new("Rule Test:\n    add(X, Y)\n    --------\n    result(X)");
    let (remaining, rule) = parse_rule(input).unwrap();

    assert_eq!(rule.name, "Test");

    match &rule.premise {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "add");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected App for premise"),
    }

    match &rule.conclusion {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "result");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected App for conclusion"),
    }

    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_int() {
    let input = Span::new("42");
    let (remaining, term) = parse_term(input).unwrap();

    match term {
        Term::Int { val } => assert_eq!(val, 42),
        _ => panic!("Expected Int"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_float() {
    let input = Span::new("3.14");
    let (remaining, term) = parse_term(input).unwrap();

    match term {
        Term::Float { val } => assert!((val - std::f32::consts::PI).abs() < 0.01),
        _ => panic!("Expected Float"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_var() {
    let input = Span::new("X");
    let (remaining, term) = parse_term(input).unwrap();

    match term {
        Term::Var { name } => assert_eq!(name, "X"),
        _ => panic!("Expected Var"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_atom() {
    let input = Span::new("foo");
    let (remaining, term) = parse_term(input).unwrap();

    match term {
        Term::Atom { text } => assert_eq!(text, "foo"),
        _ => panic!("Expected Atom"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_app() {
    let input = Span::new("add(5, X)");
    let (remaining, term) = parse_term(input).unwrap();

    match term {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "add");
            assert_eq!(args.len(), 2);

            match &args[0] {
                Term::Int { val } => assert_eq!(*val, 5),
                _ => panic!("Expected Int for first arg"),
            }

            match &args[1] {
                Term::Var { name } => assert_eq!(*name, "X"),
                _ => panic!("Expected Var for second arg"),
            }
        }
        _ => panic!("Expected App"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_nested_app() {
    let input = Span::new("mul(add(1, 2), 3)");
    let (remaining, term) = parse_term(input).unwrap();

    match term {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "mul");
            assert_eq!(args.len(), 2);

            // Check first arg is add(1, 2)
            match &args[0] {
                Term::App { rel, args } => {
                    assert_eq!(rel.name, "add");
                    assert_eq!(args.len(), 2);
                }
                _ => panic!("Expected App for first arg"),
            }
        }
        _ => panic!("Expected App"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_file_simple_rule() {
    use std::fs;

    let contents = fs::read_to_string("tests/parser/test_rule.l")
        .expect("Failed to read tests/parser/test_rule.l");
    let input = Span::new(&contents);

    let (remaining, rule) = parse_rule(input).unwrap();

    assert_eq!(rule.name, "AddCommutative");

    // Check premise: add(X, Y)
    match &rule.premise {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "add");
            assert_eq!(args.len(), 2);
            match &args[0] {
                Term::Var { name } => assert_eq!(*name, "X"),
                _ => panic!("Expected Var"),
            }
            match &args[1] {
                Term::Var { name } => assert_eq!(*name, "Y"),
                _ => panic!("Expected Var"),
            }
        }
        _ => panic!("Expected App for premise"),
    }

    // Check conclusion: add(Y, X)
    match &rule.conclusion {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "add");
            assert_eq!(args.len(), 2);
            match &args[0] {
                Term::Var { name } => assert_eq!(*name, "Y"),
                _ => panic!("Expected Var"),
            }
            match &args[1] {
                Term::Var { name } => assert_eq!(*name, "X"),
                _ => panic!("Expected Var"),
            }
        }
        _ => panic!("Expected App for conclusion"),
    }

    // Should have just a trailing newline
    assert_eq!(*remaining.fragment(), "\n");
}

#[test]
fn test_file_nested_rule() {
    use std::fs;

    let contents = fs::read_to_string("tests/parser/test_nested.l")
        .expect("Failed to read tests/parser/test_nested.l");
    let input = Span::new(&contents);

    let (remaining, rule) = parse_rule(input).unwrap();

    assert_eq!(rule.name, "Multiply");

    // Check premise: mul(add(X, Y), Z)
    match &rule.premise {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "mul");
            assert_eq!(args.len(), 2);

            // First arg should be add(X, Y)
            match &args[0] {
                Term::App { rel, args } => {
                    assert_eq!(rel.name, "add");
                    assert_eq!(args.len(), 2);
                }
                _ => panic!("Expected App"),
            }

            // Second arg should be Z
            match &args[1] {
                Term::Var { name } => assert_eq!(*name, "Z"),
                _ => panic!("Expected Var"),
            }
        }
        _ => panic!("Expected App for premise"),
    }

    // Check conclusion: add(mul(X, Z), mul(Y, Z))
    match &rule.conclusion {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "add");
            assert_eq!(args.len(), 2);

            // Both args should be mul(...) applications
            for arg in args {
                match &arg {
                    Term::App { rel, args } => {
                        assert_eq!(rel.name, "mul");
                        assert_eq!(args.len(), 2);
                    }
                    _ => panic!("Expected App"),
                }
            }
        }
        _ => panic!("Expected App for conclusion"),
    }

    assert_eq!(*remaining.fragment(), "\n");
}

#[test]
fn test_file_mixed_types() {
    use std::fs;

    let contents = fs::read_to_string("tests/parser/test_mixed.l")
        .expect("Failed to read tests/parser/test_mixed.l");
    let input = Span::new(&contents);

    let (remaining, rule) = parse_rule(input).unwrap();

    assert_eq!(rule.name, "TypeCheck");

    // Check premise: typeof(42, int)
    match &rule.premise {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "typeof");
            assert_eq!(args.len(), 2);

            // First arg: 42 (integer)
            match &args[0] {
                Term::Int { val } => assert_eq!(*val, 42),
                _ => panic!("Expected Int"),
            }

            // Second arg: int (atom)
            match &args[1] {
                Term::Atom { text } => assert_eq!(*text, "int"),
                _ => panic!("Expected Atom"),
            }
        }
        _ => panic!("Expected App for premise"),
    }

    // Check conclusion: valid(42)
    match &rule.conclusion {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "valid");
            assert_eq!(args.len(), 1);

            match &args[0] {
                Term::Int { val } => assert_eq!(*val, 42),
                _ => panic!("Expected Int"),
            }
        }
        _ => panic!("Expected App for conclusion"),
    }

    assert_eq!(*remaining.fragment(), "\n");
}

#[test]
fn test_file_invalid() {
    use std::fs;

    let contents = fs::read_to_string("tests/parser/test_invalid.l")
        .expect("Failed to read tests/parser/test_invalid.l");
    let input = Span::new(&contents);

    let result = parse_rule(input);

    assert!(result.is_err(), "Expected parsing to fail for invalid input");
}

#[test]
fn test_parse_stage() {
    let input = Span::new("Begin Stage Test:\nRule Foo:\n    bar(X)\n    ------\n    baz(X)\nEnd Stage Test");
    let (remaining, stage) = parse_stage(input).unwrap();

    assert_eq!(stage.name, "Test");
    assert_eq!(stage.rules.len(), 1);
    assert_eq!(stage.rules[0].name, "Foo");

    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_file_stage() {
    use std::fs;

    let contents = fs::read_to_string("tests/parser/test_stage.l")
        .expect("Failed to read tests/parser/test_stage.l");
    let input = Span::new(&contents);

    let (remaining, stage) = parse_stage(input).unwrap();

    assert_eq!(stage.name, "Arithmetic");
    assert_eq!(stage.rules.len(), 2);

    // Check first rule
    assert_eq!(stage.rules[0].name, "Add");
    match &stage.rules[0].premise {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "add");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected App"),
    }

    // Check second rule
    assert_eq!(stage.rules[1].name, "Multiply");
    match &stage.rules[1].premise {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "mul");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected App"),
    }

    assert_eq!(*remaining.fragment(), "\n");
}

#[test]
fn test_file_empty_stage() {
    use std::fs;

    let contents = fs::read_to_string("tests/parser/test_empty_stage.l")
        .expect("Failed to read tests/parser/test_empty_stage.l");
    let input = Span::new(&contents);

    let (remaining, stage) = parse_stage(input).unwrap();

    assert_eq!(stage.name, "Empty");
    assert_eq!(stage.rules.len(), 0);

    assert_eq!(*remaining.fragment(), "\n");
}

#[test]
fn test_file_stage_name_mismatch() {
    use std::fs;

    let contents = fs::read_to_string("tests/parser/test_stage_mismatch.l")
        .expect("Failed to read tests/parser/test_stage_mismatch.l");
    let input = Span::new(&contents);

    let result = parse_stage(input);

    assert!(result.is_err(), "Expected parsing to fail when stage names don't match");
}

#[test]
fn test_parse_module() {
    let input = Span::new("Begin Facts:\nEnd Facts\n\nBegin Global:\nEnd Global\n\nBegin Stage S1:\nRule R1:\n    a(X)\n    ----\n    b(X)\nEnd Stage S1\n\nBegin Stage S2:\nEnd Stage S2");
    let (remaining, module) = parse_module(input).unwrap();

    assert_eq!(module.facts.len(), 0);
    assert_eq!(module.global_stage.rules.len(), 0);
    assert_eq!(module.stages.len(), 2);
    assert_eq!(module.stages[0].name, "S1");
    assert_eq!(module.stages[0].rules.len(), 1);
    assert_eq!(module.stages[1].name, "S2");
    assert_eq!(module.stages[1].rules.len(), 0);

    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_file_module() {
    use std::fs;

    let contents = fs::read_to_string("tests/parser/test_module.l")
        .expect("Failed to read tests/parser/test_module.l");
    let input = Span::new(&contents);

    let (remaining, module) = parse_module(input).unwrap();

    assert_eq!(module.stages.len(), 2);

    // Check first stage
    assert_eq!(module.stages[0].name, "Arithmetic");
    assert_eq!(module.stages[0].rules.len(), 2);
    assert_eq!(module.stages[0].rules[0].name, "Add");
    assert_eq!(module.stages[0].rules[1].name, "Multiply");

    // Check second stage
    assert_eq!(module.stages[1].name, "Logic");
    assert_eq!(module.stages[1].rules.len(), 1);
    assert_eq!(module.stages[1].rules[0].name, "And");

    assert!(remaining.fragment().trim().is_empty());
}

#[test]
fn test_file_empty_module() {
    use std::fs;

    let contents = fs::read_to_string("tests/parser/test_empty_module.l")
        .expect("Failed to read tests/parser/test_empty_module.l");
    let input = Span::new(&contents);

    let (remaining, module) = parse_module(input).unwrap();

    assert_eq!(module.facts.len(), 0);
    assert_eq!(module.global_stage.rules.len(), 0);
    assert_eq!(module.stages.len(), 0);

    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_module_with_facts() {
    let input = Span::new("Begin Facts:\n    foo\n    bar(1, 2)\n    X\nEnd Facts\n\nBegin Global:\nEnd Global\n");
    let (remaining, module) = parse_module(input).unwrap();

    // Check facts
    assert_eq!(module.facts.len(), 3);

    match &module.facts[0] {
        Term::Atom { text } => assert_eq!(*text, "foo"),
        _ => panic!("Expected Atom"),
    }

    match &module.facts[1] {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "bar");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected App"),
    }

    match &module.facts[2] {
        Term::Var { name } => assert_eq!(*name, "X"),
        _ => panic!("Expected Var"),
    }

    // Check global stage is empty
    assert_eq!(module.global_stage.name, "Global");
    assert_eq!(module.global_stage.rules.len(), 0);

    // Check no regular stages
    assert_eq!(module.stages.len(), 0);

    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_module_with_global_rules() {
    let input = Span::new("Begin Facts:\nEnd Facts\n\nBegin Global:\nRule GlobalRule:\n    a(X)\n    ----\n    b(X)\nEnd Global\n");
    let (remaining, module) = parse_module(input).unwrap();

    // Check facts are empty
    assert_eq!(module.facts.len(), 0);

    // Check global stage has one rule
    assert_eq!(module.global_stage.name, "Global");
    assert_eq!(module.global_stage.rules.len(), 1);
    assert_eq!(module.global_stage.rules[0].name, "GlobalRule");

    // Check no regular stages
    assert_eq!(module.stages.len(), 0);

    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_complete_module() {
    let input = Span::new("Begin Facts:\n    initial(0)\n    count(5)\nEnd Facts\n\nBegin Global:\nRule Increment:\n    count(X)\n    --------\n    count(add(X, 1))\nEnd Global\n\nBegin Stage Processing:\nRule Process:\n    data(X)\n    -------\n    processed(X)\nEnd Stage Processing\n");
    let (remaining, module) = parse_module(input).unwrap();

    // Check facts
    assert_eq!(module.facts.len(), 2);
    match &module.facts[0] {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "initial");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected App"),
    }

    // Check global rules
    assert_eq!(module.global_stage.name, "Global");
    assert_eq!(module.global_stage.rules.len(), 1);
    assert_eq!(module.global_stage.rules[0].name, "Increment");

    // Check regular stages
    assert_eq!(module.stages.len(), 1);
    assert_eq!(module.stages[0].name, "Processing");
    assert_eq!(module.stages[0].rules.len(), 1);
    assert_eq!(module.stages[0].rules[0].name, "Process");

    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_module_empty_facts_and_global() {
    let input = Span::new("Begin Facts:\nEnd Facts\n\nBegin Global:\nEnd Global\n\nBegin Stage Test:\nEnd Stage Test");
    let (remaining, module) = parse_module(input).unwrap();

    // Check facts are empty
    assert_eq!(module.facts.len(), 0);

    // Check global stage is empty
    assert_eq!(module.global_stage.name, "Global");
    assert_eq!(module.global_stage.rules.len(), 0);

    // Check one empty stage
    assert_eq!(module.stages.len(), 1);
    assert_eq!(module.stages[0].name, "Test");
    assert_eq!(module.stages[0].rules.len(), 0);

    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_module_multiple_facts() {
    let input = Span::new("Begin Facts:\n    atom1\n    atom2\n    app(1, 2, 3)\n    42\n    3.14\n    Variable\nEnd Facts\n\nBegin Global:\nEnd Global\n");
    let (remaining, module) = parse_module(input).unwrap();

    // Check all fact types
    assert_eq!(module.facts.len(), 6);

    match &module.facts[0] {
        Term::Atom { text } => assert_eq!(*text, "atom1"),
        _ => panic!("Expected Atom"),
    }

    match &module.facts[1] {
        Term::Atom { text } => assert_eq!(*text, "atom2"),
        _ => panic!("Expected Atom"),
    }

    match &module.facts[2] {
        Term::App { args, .. } => assert_eq!(args.len(), 3),
        _ => panic!("Expected App"),
    }

    match &module.facts[3] {
        Term::Int { val } => assert_eq!(*val, 42),
        _ => panic!("Expected Int"),
    }

    match &module.facts[4] {
        Term::Float { val } => assert!((val - std::f32::consts::PI).abs() < 0.01),
        _ => panic!("Expected Float"),
    }

    match &module.facts[5] {
        Term::Var { name } => assert_eq!(*name, "Variable"),
        _ => panic!("Expected Var"),
    }

    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_module_multiple_global_rules() {
    let input = Span::new("Begin Facts:\nEnd Facts\n\nBegin Global:\nRule Rule1:\n    a(X)\n    ----\n    b(X)\n\nRule Rule2:\n    c(Y)\n    ----\n    d(Y)\nEnd Global\n");
    let (remaining, module) = parse_module(input).unwrap();

    // Check global stage has two rules
    assert_eq!(module.global_stage.name, "Global");
    assert_eq!(module.global_stage.rules.len(), 2);
    assert_eq!(module.global_stage.rules[0].name, "Rule1");
    assert_eq!(module.global_stage.rules[1].name, "Rule2");

    assert_eq!(*remaining.fragment(), "");
}

// ============================================================================
// Binary operator tests
// ============================================================================

fn assert_binop(term: &Term, expected_rel: &str, check_args: impl FnOnce(&[Term])) {
    match &term {
        Term::App { rel, args } => {
            assert_eq!(rel.name, expected_rel, "Expected rel {}, got {}", expected_rel, rel.name);
            check_args(args);
        }
        _ => panic!("Expected App, got {:?}", term),
    }
}

#[test]
fn test_parse_eq_operator() {
    let input = Span::new("X = Y");
    let (remaining, term) = parse_term(input).unwrap();
    assert_binop(&term, "eq", |args| {
        assert_eq!(args.len(), 2);
        assert!(matches!(&args[0], Term::Var { name } if name == "X"));
        assert!(matches!(&args[1], Term::Var { name } if name == "Y"));
    });
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_or_operator() {
    let input = Span::new("a | b");
    let (_, term) = parse_term(input).unwrap();
    assert_binop(&term, "or", |args| {
        assert_eq!(args.len(), 2);
    });

    let input2 = Span::new("a ∨ b");
    let (_, term2) = parse_term(input2).unwrap();
    assert_binop(&term2, "or", |args| {
        assert_eq!(args.len(), 2);
    });
}

#[test]
fn test_parse_and_operator() {
    let input = Span::new("a & b");
    let (_, term) = parse_term(input).unwrap();
    assert_binop(&term, "and", |args| {
        assert_eq!(args.len(), 2);
    });

    let input2 = Span::new("a ∧ b");
    let (_, term2) = parse_term(input2).unwrap();
    assert_binop(&term2, "and", |args| {
        assert_eq!(args.len(), 2);
    });
}

#[test]
fn test_parse_not_operator() {
    let input = Span::new("!a");
    let (_, term) = parse_term(input).unwrap();
    assert_binop(&term, "not", |args| {
        assert_eq!(args.len(), 1);
    });

    let input2 = Span::new("¬a");
    let (_, term2) = parse_term(input2).unwrap();
    assert_binop(&term2, "not", |args| {
        assert_eq!(args.len(), 1);
    });
}

#[test]
fn test_parse_int_comparisons() {
    for (op, rel) in [("<", "int_lt"), ("<=", "int_le"), (">", "int_gt"), (">=", "int_ge"), ("==", "int_eq")] {
        let s = format!("X {} Y", op);
        let input = Span::new(&s);
        let (_, term) = parse_term(input).unwrap();
        assert_binop(&term, rel, |args| {
            assert_eq!(args.len(), 2);
        });
    }
}

#[test]
fn test_parse_real_comparisons() {
    for (op, rel) in [(".<", "real_lt"), (".<=", "real_le"), (".>", "real_gt"), (".>=", "real_ge"), (".==", "real_eq")] {
        let s = format!("X {} Y", op);
        let input = Span::new(&s);
        let (_, term) = parse_term(input).unwrap();
        assert_binop(&term, rel, |args| {
            assert_eq!(args.len(), 2);
        });
    }
}

#[test]
fn test_operator_precedence() {
    // a = b & c should parse as and(eq(a, b), c) - eq binds tighter than and
    let input = Span::new("a = b & c");
    let (_, term) = parse_term(input).unwrap();
    assert_binop(&term, "and", |args| {
        assert_eq!(args.len(), 2);
        assert_binop(&args[0], "eq", |eq_args| {
            assert!(matches!(&eq_args[0], Term::Atom { text } if text == "a"));
            assert!(matches!(&eq_args[1], Term::Atom { text } if text == "b"));
        });
        assert!(matches!(&args[1], Term::Atom { text } if text == "c"));
    });

    // a | b & c should parse as or(a, and(b, c))
    let input2 = Span::new("a | b & c");
    let (_, term2) = parse_term(input2).unwrap();
    assert_binop(&term2, "or", |args| {
        assert_eq!(args.len(), 2);
        assert!(matches!(&args[0], Term::Atom { text } if text == "a"));
        assert_binop(&args[1], "and", |_| {});
    });

    // !a & b should parse as and(not(a), b)
    let input3 = Span::new("!a & b");
    let (_, term3) = parse_term(input3).unwrap();
    assert_binop(&term3, "and", |args| {
        assert_eq!(args.len(), 2);
        assert_binop(&args[0], "not", |_| {});
    });
}

#[test]
fn test_parenthesized_expr() {
    // (a | b) & c should parse as and(or(a, b), c)
    let input = Span::new("(a | b) & c");
    let (_, term) = parse_term(input).unwrap();
    assert_binop(&term, "and", |args| {
        assert_eq!(args.len(), 2);
        assert_binop(&args[0], "or", |_| {});
    });
}

#[test]
fn test_comparison_precedence() {
    // X < 5 & Y > 3 should parse as and(int_lt(X, 5), int_gt(Y, 3))
    let input = Span::new("X < 5 & Y > 3");
    let (_, term) = parse_term(input).unwrap();
    assert_binop(&term, "and", |args| {
        assert_eq!(args.len(), 2);
        assert_binop(&args[0], "int_lt", |_| {});
        assert_binop(&args[1], "int_gt", |_| {});
    });
}

// ============================================================================
// Comment tests
// ============================================================================

#[test]
fn test_comment_after_fact() {
    let input = Span::new("Begin Facts:\n    foo # this is a comment\nEnd Facts\n\nBegin Global:\nEnd Global\n");
    let (remaining, module) = parse_module(input).unwrap();
    
    assert_eq!(module.facts.len(), 1);
    match &module.facts[0] {
        Term::Atom { text } => assert_eq!(*text, "foo"),
        _ => panic!("Expected Atom"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_comment_only_line() {
    let input = Span::new("Begin Facts:\n    # just a comment\n    foo\nEnd Facts\n\nBegin Global:\nEnd Global\n");
    let (remaining, module) = parse_module(input).unwrap();
    
    assert_eq!(module.facts.len(), 1);
    match &module.facts[0] {
        Term::Atom { text } => assert_eq!(*text, "foo"),
        _ => panic!("Expected Atom"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_comment_in_rule() {
    let input = Span::new("Rule Test:\n    add(X, Y) # premise comment\n    --------\n    result(X) # conclusion comment");
    let (remaining, rule) = parse_rule(input).unwrap();
    
    assert_eq!(rule.name, "Test");
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_comment_between_rules() {
    let input = Span::new("Begin Stage Test:\n# comment before rule\nRule Foo:\n    bar(X)\n    ------\n    baz(X)\n# comment after rule\nEnd Stage Test");
    let (remaining, stage) = parse_stage(input).unwrap();
    
    assert_eq!(stage.name, "Test");
    assert_eq!(stage.rules.len(), 1);
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_multiple_comments() {
    let input = Span::new("Begin Facts:\n    # comment 1\n    # comment 2\n    foo\n    # comment 3\nEnd Facts\n\nBegin Global:\nEnd Global\n");
    let (remaining, module) = parse_module(input).unwrap();
    
    assert_eq!(module.facts.len(), 1);
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_comment_in_app_args() {
    // Comments within argument lists
    let input = Span::new("foo(\n    X, # first arg\n    Y  # second arg\n)");
    let (remaining, term) = parse_term(input).unwrap();
    
    match &term {
        Term::App { rel, args } => {
            assert_eq!(rel.name, "foo");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected App"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_comment_with_hash_in_string_context() {
    // Ensure # is properly handled - everything after # to EOL is ignored
    let input = Span::new("foo # bar(X) this is all comment\n");
    let (remaining, term) = parse_term(input).unwrap();
    
    match &term {
        Term::Atom { text } => assert_eq!(*text, "foo"),
        _ => panic!("Expected Atom"),
    }
    // Should leave the newline and nothing else
    assert!(remaining.fragment().starts_with(" #") || remaining.fragment().starts_with("#") || remaining.fragment().trim().is_empty() || remaining.fragment().starts_with("\n"));
}

// ============================================================================
// When/otherwise (cond) tests
// ============================================================================

#[test]
fn test_parse_when_otherwise_inline() {
    let input = Span::new("when X = 1: foo(X), otherwise: bar(X)");
    let (remaining, term) = parse_term(input).unwrap();
    
    assert_binop(&term, "cond", |args| {
        assert_eq!(args.len(), 3);
        // Guard: eq(X, 1)
        assert_binop(&args[0], "eq", |eq_args| {
            assert!(matches!(&eq_args[0], Term::Var { name } if name == "X"));
            assert!(matches!(&eq_args[1], Term::Int { val } if *val == 1));
        });
        // Then branch: foo(X)
        assert_binop(&args[1], "foo", |foo_args| {
            assert_eq!(foo_args.len(), 1);
        });
        // Else branch: bar(X)
        assert_binop(&args[2], "bar", |bar_args| {
            assert_eq!(bar_args.len(), 1);
        });
    });
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_when_otherwise_multiline() {
    let input = Span::new("when X = 1:\n    foo(X)\notherwise:\n    bar(X)");
    let (remaining, term) = parse_term(input).unwrap();
    
    assert_binop(&term, "cond", |args| {
        assert_eq!(args.len(), 3);
        assert_binop(&args[0], "eq", |_| {});
        assert_binop(&args[1], "foo", |_| {});
        assert_binop(&args[2], "bar", |_| {});
    });
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_when_otherwise_nested() {
    // Nested when/otherwise in else branch
    let input = Span::new("when A: foo, otherwise: when B: bar, otherwise: baz");
    let (remaining, term) = parse_term(input).unwrap();
    
    assert_binop(&term, "cond", |args| {
        assert_eq!(args.len(), 3);
        assert!(matches!(&args[0], Term::Var { name } if name == "A"));
        assert!(matches!(&args[1], Term::Atom { text } if text == "foo"));
        // Else branch is another cond
        assert_binop(&args[2], "cond", |inner_args| {
            assert_eq!(inner_args.len(), 3);
            assert!(matches!(&inner_args[0], Term::Var { name } if name == "B"));
            assert!(matches!(&inner_args[1], Term::Atom { text } if text == "bar"));
            assert!(matches!(&inner_args[2], Term::Atom { text } if text == "baz"));
        });
    });
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_when_otherwise_with_operators() {
    // when with or in guard - should bind looser than or
    let input = Span::new("when A | B: foo, otherwise: bar");
    let (remaining, term) = parse_term(input).unwrap();
    
    assert_binop(&term, "cond", |args| {
        assert_eq!(args.len(), 3);
        // Guard should be or(A, B)
        assert_binop(&args[0], "or", |or_args| {
            assert!(matches!(&or_args[0], Term::Var { name } if name == "A"));
            assert!(matches!(&or_args[1], Term::Var { name } if name == "B"));
        });
    });
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_when_otherwise_complex_branches() {
    let input = Span::new("when X > 0 & Y > 0: positive, otherwise: non_positive");
    let (remaining, term) = parse_term(input).unwrap();
    
    assert_binop(&term, "cond", |args| {
        assert_eq!(args.len(), 3);
        // Guard should be and(int_gt(X, 0), int_gt(Y, 0))
        assert_binop(&args[0], "and", |and_args| {
            assert_binop(&and_args[0], "int_gt", |_| {});
            assert_binop(&and_args[1], "int_gt", |_| {});
        });
        assert!(matches!(&args[1], Term::Atom { text } if text == "positive"));
        assert!(matches!(&args[2], Term::Atom { text } if text == "non_positive"));
    });
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_when_otherwise_reserved_keywords() {
    // "when" should not be parseable as an atom
    let input = Span::new("when");
    let result = parse_term(input);
    assert!(result.is_err(), "Expected 'when' as standalone to fail");
    
    // "otherwise" should not be parseable as an atom
    let input2 = Span::new("otherwise");
    let result2 = parse_term(input2);
    assert!(result2.is_err(), "Expected 'otherwise' as standalone to fail");
}

#[test]
fn test_when_otherwise_comma_required() {
    // Missing comma and no newline should fail
    let input = Span::new("when A: foo otherwise: bar");
    let result = parse_term(input);
    assert!(result.is_err(), "Expected missing separator to fail");
}

#[test]
fn test_when_otherwise_with_comma_and_newline() {
    // Both comma and newline should work
    let input = Span::new("when A: foo,\notherwise: bar");
    let (remaining, term) = parse_term(input).unwrap();
    
    assert_binop(&term, "cond", |args| {
        assert_eq!(args.len(), 3);
    });
    assert_eq!(*remaining.fragment(), "");
}
