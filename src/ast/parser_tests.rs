use super::parser::*;
use super::*;

#[test]
fn test_parse_rule() {
    let input = Span::new("Rule Test:\n    add(X, Y)\n    --------\n    result(X)");
    let (remaining, rule) = parse_rule(input).unwrap();

    assert_eq!(rule.name, "Test");

    // Check premise
    match &rule.premise.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(*name, "add"),
                _ => panic!("Expected UserRel"),
            }
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected App for premise"),
    }

    // Check conclusion
    match &rule.conclusion.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(*name, "result"),
                _ => panic!("Expected UserRel"),
            }
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

    match term.contents {
        TermContents::Int { val } => assert_eq!(val, 42),
        _ => panic!("Expected Int"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_float() {
    let input = Span::new("3.14");
    let (remaining, term) = parse_term(input).unwrap();

    match term.contents {
        TermContents::Float { val } => assert!((val - 3.14).abs() < 0.001),
        _ => panic!("Expected Float"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_var() {
    let input = Span::new("X");
    let (remaining, term) = parse_term(input).unwrap();

    match term.contents {
        TermContents::Var { name } => assert_eq!(name, "X"),
        _ => panic!("Expected Var"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_atom() {
    let input = Span::new("foo");
    let (remaining, term) = parse_term(input).unwrap();

    match term.contents {
        TermContents::Atom { text } => assert_eq!(text, "foo"),
        _ => panic!("Expected Atom"),
    }
    assert_eq!(*remaining.fragment(), "");
}

#[test]
fn test_parse_app() {
    let input = Span::new("add(5, X)");
    let (remaining, term) = parse_term(input).unwrap();

    match term.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(name, "add"),
                _ => panic!("Expected UserRel"),
            }
            assert_eq!(args.len(), 2);

            match &args[0].contents {
                TermContents::Int { val } => assert_eq!(*val, 5),
                _ => panic!("Expected Int for first arg"),
            }

            match &args[1].contents {
                TermContents::Var { name } => assert_eq!(*name, "X"),
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

    match term.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(name, "mul"),
                _ => panic!("Expected UserRel"),
            }
            assert_eq!(args.len(), 2);

            // Check first arg is add(1, 2)
            match &args[0].contents {
                TermContents::App { rel, args } => {
                    match rel {
                        Rel::UserRel { name } => assert_eq!(*name, "add"),
                        _ => panic!("Expected UserRel"),
                    }
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
    match &rule.premise.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(*name, "add"),
                _ => panic!("Expected UserRel"),
            }
            assert_eq!(args.len(), 2);
            match &args[0].contents {
                TermContents::Var { name } => assert_eq!(*name, "X"),
                _ => panic!("Expected Var"),
            }
            match &args[1].contents {
                TermContents::Var { name } => assert_eq!(*name, "Y"),
                _ => panic!("Expected Var"),
            }
        }
        _ => panic!("Expected App for premise"),
    }

    // Check conclusion: add(Y, X)
    match &rule.conclusion.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(*name, "add"),
                _ => panic!("Expected UserRel"),
            }
            assert_eq!(args.len(), 2);
            match &args[0].contents {
                TermContents::Var { name } => assert_eq!(*name, "Y"),
                _ => panic!("Expected Var"),
            }
            match &args[1].contents {
                TermContents::Var { name } => assert_eq!(*name, "X"),
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
    match &rule.premise.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(*name, "mul"),
                _ => panic!("Expected UserRel"),
            }
            assert_eq!(args.len(), 2);

            // First arg should be add(X, Y)
            match &args[0].contents {
                TermContents::App { rel, args } => {
                    match rel {
                        Rel::UserRel { name } => assert_eq!(*name, "add"),
                        _ => panic!("Expected UserRel"),
                    }
                    assert_eq!(args.len(), 2);
                }
                _ => panic!("Expected App"),
            }

            // Second arg should be Z
            match &args[1].contents {
                TermContents::Var { name } => assert_eq!(*name, "Z"),
                _ => panic!("Expected Var"),
            }
        }
        _ => panic!("Expected App for premise"),
    }

    // Check conclusion: add(mul(X, Z), mul(Y, Z))
    match &rule.conclusion.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(*name, "add"),
                _ => panic!("Expected UserRel"),
            }
            assert_eq!(args.len(), 2);

            // Both args should be mul(...) applications
            for arg in args {
                match &arg.contents {
                    TermContents::App { rel, args } => {
                        match rel {
                            Rel::UserRel { name } => assert_eq!(*name, "mul"),
                            _ => panic!("Expected UserRel"),
                        }
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
    match &rule.premise.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(*name, "typeof"),
                _ => panic!("Expected UserRel"),
            }
            assert_eq!(args.len(), 2);

            // First arg: 42 (integer)
            match &args[0].contents {
                TermContents::Int { val } => assert_eq!(*val, 42),
                _ => panic!("Expected Int"),
            }

            // Second arg: int (atom)
            match &args[1].contents {
                TermContents::Atom { text } => assert_eq!(*text, "int"),
                _ => panic!("Expected Atom"),
            }
        }
        _ => panic!("Expected App for premise"),
    }

    // Check conclusion: valid(42)
    match &rule.conclusion.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(*name, "valid"),
                _ => panic!("Expected UserRel"),
            }
            assert_eq!(args.len(), 1);

            match &args[0].contents {
                TermContents::Int { val } => assert_eq!(*val, 42),
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
    match &stage.rules[0].premise.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(*name, "add"),
                _ => panic!("Expected UserRel"),
            }
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected App"),
    }

    // Check second rule
    assert_eq!(stage.rules[1].name, "Multiply");
    match &stage.rules[1].premise.contents {
        TermContents::App { rel, args } => {
            match rel {
                Rel::UserRel { name } => assert_eq!(*name, "mul"),
                _ => panic!("Expected UserRel"),
            }
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
    let input = Span::new("Begin Stage S1:\nRule R1:\n    a(X)\n    ----\n    b(X)\nEnd Stage S1\n\nBegin Stage S2:\nEnd Stage S2");
    let (remaining, module) = parse_module(input).unwrap();

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

    assert_eq!(module.stages.len(), 0);

    assert_eq!(*remaining.fragment(), "");
}
