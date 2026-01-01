use crate::ast::compile::*;
use crate::ast::*;
use crate::solver::ir::*;
use nom::Finish;

fn parse_and_compile(input: &str) -> Program {
    let result = parser::parse_module(input.into()).finish();
    let (_, module) = result.expect("parse failed");
    compile(&module)
}

#[test]
fn test_simple_fact() {
    let input = r#"Begin Facts:
position(player, 0, 0)
End Facts

Begin Global:
End Global
"#;
    let program = parse_and_compile(input);
    assert_eq!(program.facts.len(), 1);
    let fact_prop = program.props.get(program.facts[0]);
    match fact_prop {
        Prop::App { rel, args } => {
            assert_eq!(program.rels.get(*rel).name, "position");
            assert_eq!(args.len(), 3);
        }
        _ => panic!("Expected Prop::App"),
    }
}

#[test]
fn test_simple_rule() {
    let input = r#"Begin Facts:
End Facts

Begin Global:
Rule MoveRight:
position(player, X, Y)
----------------------
position(player, X, Y)
End Global
"#;
    let program = parse_and_compile(input);
    let clause = program.global_rules.iter().find(|c| c.name == "MoveRight").unwrap();
    assert_eq!(program.rels.get(clause.head_rel).name, "position");
}

#[test]
fn test_smt_relation_in_rule() {
    let input = r#"Begin Facts:
End Facts

Begin Global:
Rule Increment:
int_add(X, 1, Y)
----------------
count(Y)
End Global
"#;
    let program = parse_and_compile(input);

    let clause = program.global_rules.iter().find(|c| c.name == "Increment").unwrap();
    let body_prop = program.props.get(clause.body);
    match body_prop {
        Prop::Constraint { kind, args } => {
            assert_eq!(*kind, ConstraintKind::IntAdd);
            assert_eq!(args.len(), 3);
        }
        _ => panic!("Expected Constraint prop"),
    }
}

#[test]
fn test_stage_compilation() {
    let input = r#"Begin Facts:
End Facts

Begin Global:
End Global

Begin Stage Movement:
Rule Left:
position(X)
-----------
moved(X)
End Stage Movement
"#;
    let program = parse_and_compile(input);
    assert_eq!(program.stages.len(), 1);
    assert_eq!(program.stages[0].name, "Movement");
    assert_eq!(program.stages[0].rules.len(), 1);
}
