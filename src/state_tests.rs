#[cfg(test)]
mod tests {
    use crate::Frontend;

    #[test]
    fn test_state_var_declaration() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    StateVar Health
    eq(Health, 10)
End Facts

Begin Global:
End Global
"#).unwrap();

        assert_eq!(frontend.program.state_vars.len(), 1);
        assert_eq!(frontend.program.state_vars[0], "Health");
        
        let health = frontend.get_state_var("Health");
        assert!(health.is_some(), "Health should be defined");
        assert_eq!(health.unwrap(), "10");
    }

    #[test]
    fn test_multiple_state_vars() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    StateVar Health
    StateVar Score
    StateVar Lives
    eq(Health, 100)
    eq(Score, 0)
    eq(Lives, 3)
End Facts

Begin Global:
End Global
"#).unwrap();

        assert_eq!(frontend.program.state_vars.len(), 3);
        
        let vars = frontend.state_vars();
        assert_eq!(vars.len(), 3);
        
        assert_eq!(frontend.get_state_var("Health").unwrap(), "100");
        assert_eq!(frontend.get_state_var("Score").unwrap(), "0");
        assert_eq!(frontend.get_state_var("Lives").unwrap(), "3");
    }

    #[test]
    fn test_state_constraint_basic() {
        let input = std::fs::read_to_string("sample/state_basic.l")
            .expect("Failed to read sample/state_basic.l");
        
        let mut frontend = Frontend::new();
        frontend.load(&input).unwrap();

        assert_eq!(frontend.get_state_var("Health").unwrap(), "10");

        frontend.run_stage(0).expect("Stage should succeed");

        assert_eq!(frontend.get_state_var("Health").unwrap(), "9");
    }

    #[test]
    fn test_state_constraint_multiple_updates() {
        let input = std::fs::read_to_string("sample/state_multiple.l")
            .expect("Failed to read sample/state_multiple.l");
        
        let mut frontend = Frontend::new();
        frontend.load(&input).unwrap();

        assert_eq!(frontend.get_state_var("Health").unwrap(), "100");
        assert_eq!(frontend.get_state_var("Score").unwrap(), "0");

        frontend.run_stage(0).expect("Stage should succeed");

        assert_eq!(frontend.get_state_var("Health").unwrap(), "95");
        assert_eq!(frontend.get_state_var("Score").unwrap(), "10");
    }

    #[test]
    fn test_state_constraint_repeated_execution() {
        let input = std::fs::read_to_string("sample/state_basic.l")
            .expect("Failed to read sample/state_basic.l");
        
        let mut frontend = Frontend::new();
        frontend.load(&input).unwrap();

        assert_eq!(frontend.get_state_var("Health").unwrap(), "10");

        for expected in (7..=9).rev() {
            frontend.run_stage(0).expect("Stage should succeed");
            assert_eq!(
                frontend.get_state_var("Health").unwrap(), 
                expected.to_string(),
                "After decrement, Health should be {}", expected
            );
        }
    }

    #[test]
    fn test_state_constraint_ambiguous_error() {
        let input = std::fs::read_to_string("sample/state_ambiguous.l")
            .expect("Failed to read sample/state_ambiguous.l");
        
        let mut frontend = Frontend::new();
        frontend.load(&input).unwrap();

        let result = frontend.run_stage(0);
        assert!(result.is_err(), "Ambiguous state update should fail");
        
        let err = result.unwrap_err();
        assert!(err.contains("Ambiguous") || err.contains("multiple solutions"), 
            "Error should mention ambiguity: {}", err);
    }

    #[test]
    fn test_state_constraint_no_solution_error() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    StateVar X
    eq(X, 1)
End Facts

Begin Global:
End Global

Begin Stage Impossible:
Begin State Constraints:
    int_add(next(X), 0, 999)
    int_add(next(X), 0, 1)
End State Constraints
End Stage Impossible
"#).unwrap();

        let result = frontend.run_stage(0);
        assert!(result.is_err(), "Contradictory constraints should fail");
        
        let err = result.unwrap_err();
        assert!(err.contains("no solutions"), "Error should mention no solutions: {}", err);
    }

    #[test]
    fn test_run_stage_by_name() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    StateVar Counter
    eq(Counter, 0)
End Facts

Begin Global:
End Global

Begin Stage Increment:
Begin State Constraints:
    int_add(Counter, 1, next(Counter))
End State Constraints
End Stage Increment
"#).unwrap();

        assert_eq!(frontend.get_state_var("Counter").unwrap(), "0");

        frontend.run_stage_by_name("Increment").expect("Stage should succeed");

        assert_eq!(frontend.get_state_var("Counter").unwrap(), "1");
    }

    #[test]
    fn test_stage_without_constraints() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    StateVar X
    eq(X, 5)
End Facts

Begin Global:
End Global

Begin Stage NoConstraints:
Rule Dummy:
    eq(A, A)
    --------
    dummy(A)
End Stage NoConstraints
"#).unwrap();

        assert_eq!(frontend.get_state_var("X").unwrap(), "5");

        frontend.run_stage(0).expect("Empty constraint stage should succeed");

        assert_eq!(frontend.get_state_var("X").unwrap(), "5");
    }

    #[test]
    fn test_state_var_not_mentioned_preserved() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    StateVar A
    StateVar B
    eq(A, 10)
    eq(B, 20)
End Facts

Begin Global:
End Global

Begin Stage UpdateA:
Begin State Constraints:
    int_add(A, 1, next(A))
End State Constraints
End Stage UpdateA
"#).unwrap();

        assert_eq!(frontend.get_state_var("A").unwrap(), "10");
        assert_eq!(frontend.get_state_var("B").unwrap(), "20");

        frontend.run_stage(0).expect("Stage should succeed");

        assert_eq!(frontend.get_state_var("A").unwrap(), "11");
        assert_eq!(frontend.get_state_var("B").unwrap(), "20");
    }

    #[test]
    fn test_parser_state_constraints_section() {
        use crate::ast::parser;
        use nom::Finish;

        let input = r#"Begin Facts:
    StateVar X
    eq(X, 1)
End Facts

Begin Global:
End Global

Begin Stage TestStage:
Rule Dummy:
    eq(A, A)
    --------
    dummy(A)
Begin State Constraints:
    eq(X, 2)
End State Constraints
End Stage TestStage
"#;

        let result = parser::parse_module(input.into()).finish();
        assert!(result.is_ok(), "Parsing should succeed: {:?}", result);
        
        let (_, module) = result.unwrap();
        assert_eq!(module.state_vars.len(), 1);
        assert_eq!(module.stages.len(), 1);
        assert_eq!(module.stages[0].state_constraints.len(), 1);
    }
}
