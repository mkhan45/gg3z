#[cfg(test)]
mod tests {
    use crate::frontend::Frontend;

    #[test]
    fn test_position_query() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    position(player, 0, 0)
End Facts

Begin Global:
End Global
"#).unwrap();

        eprintln!("Facts: {}", frontend.program.facts.len());
        for &fact_prop_id in &frontend.program.facts {
            let fact_prop = frontend.program.props.get(fact_prop_id);
            eprintln!("  fact: {:?}", fact_prop);
        }
        eprintln!("Rels:");
        for (id, rel) in frontend.program.rels.iter() {
            eprintln!("  {:?}: {:?}", id, rel);
        }
        
        let result = frontend.query("position(player, X, Y)").unwrap();
        eprintln!("Query result: {:?}", result);
        assert!(!result.is_empty(), "Expected at least one solution");
    }

    #[test]
    fn test_eq_fact_constrains_query() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    eq(X, 1)
End Facts

Begin Global:
End Global
"#).unwrap();

        let result = frontend.query("eq(X, 2)").unwrap();
        assert!(result.is_empty(), "eq(X, 2) should fail when eq(X, 1) is a fact, got: {:?}", result);
    }

    #[test]
    fn test_eq_fact_allows_compatible_query() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    eq(X, 1)
End Facts

Begin Global:
End Global
"#).unwrap();

        let result = frontend.query("eq(X, Y)").unwrap();
        assert!(!result.is_empty(), "eq(X, Y) should succeed with Y=1 when eq(X, 1) is a fact");
    }

    #[test]
    fn test_eq_with_cons_term() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    eq(X, cons(A, B))
End Facts

Begin Global:
End Global
"#).unwrap();

        let result = frontend.query("eq(X, Y)").unwrap();
        assert!(!result.is_empty(), "eq(X, cons(A, B)) should work with relational solver");
    }

    #[test]
    fn test_eq_fact_with_rule_present() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    eq(L, pair(1, 2))
End Facts

Begin Global:
    Rule Test:
    eq(A, B)
    --------
    someThing(B, A)
End Global
"#).unwrap();

        let result = frontend.query("eq(A, L)").unwrap();
        eprintln!("Query result: {:?}", result);
        assert!(!result.is_empty(), "Expected at least one solution");
        assert!(result[0].contains("pair"), "Expected L to be bound to pair(1, 2), got: {:?}", result);
    }

    #[test]
    fn test_incremental_query() {
        let mut frontend = Frontend::new();
        frontend.load(r#"Begin Facts:
    true()
End Facts

Begin Global:
    Rule Base:
    true()
    ------
    num(1)

    Rule Step:
    num(X)
    ------
    num(s(X))
End Global
"#).unwrap();

        frontend.max_steps = 1000;

        let first = frontend.query_start("num(X)").unwrap();
        assert!(first.is_some(), "Should find first solution");
        eprintln!("First: {:?}", first);

        let second = frontend.query_next();
        assert!(second.is_some(), "Should find second solution");
        eprintln!("Second: {:?}", second);

        let third = frontend.query_next();
        assert!(third.is_some(), "Should find third solution");
        eprintln!("Third: {:?}", third);

        frontend.query_stop();
        assert!(!frontend.has_more_solutions(), "No more after stop");
    }
}
