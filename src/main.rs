pub mod ast;
pub mod ir;
pub mod solver;
mod test_samelength;
mod state_tests;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use nom::Finish;
use ast::parser;
use ast::compile::Compiler;

use ast::Module;
use ir::Program;
use solver::{format_solution, Solver, SearchStrategy, SearchQueue, Subst, State};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_module(input: *const c_char) -> *mut Module {
    unsafe {
        let inp = CStr::from_ptr(input).to_str().unwrap_or("");
        let res = parser::parse_module(inp.into()).finish();
        match res {
            Ok((_rest, module)) => Box::leak(Box::new(module)) as *mut _,
            Err(_e) => std::ptr::null::<Module>() as *mut _,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn module_to_string(module: *mut Module) -> *mut c_char {
    unsafe {
        let s = CString::new((*module).to_string()).unwrap();
        s.into_raw()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_module(module: *mut Module) {
    unsafe { std::ptr::drop_in_place(module) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn module_stage_count(module: *mut Module) -> i32 {
    unsafe { (*module).stages.len() as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn module_get_stage_name(module: *mut Module, index: i32) -> *mut c_char {
    unsafe {
        if let Some(stage) = (&(*module).stages).get(index as usize) {
            CString::new(stage.name.clone()).unwrap().into_raw()
        } else {
            std::ptr::null_mut()
        }
    }
}

pub struct Frontend {
    pub program: Program,
    pub var_map: std::collections::HashMap<String, ir::TermId>,
    pub strategy: SearchStrategy,
    pub max_steps: usize,
    pending_queue: Option<SearchQueue>,
    pending_query_vars: Vec<(String, ir::TermId)>,
}

impl Frontend {
    pub fn new() -> Self {
        Self {
            program: Program::default(),
            var_map: std::collections::HashMap::new(),
            strategy: SearchStrategy::default(),
            max_steps: 10_000,
            pending_queue: None,
            pending_query_vars: Vec::new(),
        }
    }

    pub fn load(&mut self, source: &str) -> Result<(), String> {
        let result = parser::parse_module(source.into()).finish();
        match result {
            Ok((_, module)) => {
                self.program = Program::default();
                let mut compiler = Compiler::new(&mut self.program);
                compiler.compile_module(&module);
                self.var_map = compiler.into_var_map();
                Ok(())
            }
            Err(e) => Err(format!("Parse error: {:?}", e)),
        }
    }

    pub fn query(&mut self, query_str: &str) -> Result<Vec<String>, String> {
        self.query_with_limit(query_str, 10)
    }

    pub fn query_with_limit(&mut self, query_str: &str, limit: usize) -> Result<Vec<String>, String> {
        self.query_with_limit_and_steps(query_str, limit, self.max_steps)
    }

    pub fn query_with_limit_and_steps(&mut self, query_str: &str, limit: usize, max_steps: usize) -> Result<Vec<String>, String> {
        let term_result = parser::parse_term(query_str.into()).finish();
        let term = match term_result {
            Ok((_, term)) => term,
            Err(e) => return Err(format!("Query parse error: {:?}", e)),
        };

        let (goal, query_vars) = Compiler::with_var_map(&mut self.program, self.var_map.clone())
            .compile_query(&term);

        let mut solver = Solver::new(&mut self.program);
        let solutions: Vec<_> = solver.query_with_strategy(goal, self.strategy)
            .with_limit(limit)
            .with_max_steps(max_steps)
            .collect();

        Ok(solutions
            .iter()
            .map(|s| format_solution(&query_vars, s, solver.program))
            .collect())
    }

    pub fn query_start(&mut self, query_str: &str) -> Result<Option<String>, String> {
        let term_result = parser::parse_term(query_str.into()).finish();
        let term = match term_result {
            Ok((_, term)) => term,
            Err(e) => return Err(format!("Query parse error: {:?}", e)),
        };

        let (goal, query_vars) = Compiler::with_var_map(&mut self.program, self.var_map.clone())
            .compile_query(&term);

        self.pending_query_vars = query_vars;

        let mut solver = Solver::new(&mut self.program);
        let queue = solver.init_query(goal, self.strategy);
        let (solution, remaining_queue) = solver.step_until_solution(queue, self.max_steps);

        self.pending_queue = Some(remaining_queue);

        match solution {
            Some(state) => Ok(Some(format_solution(&self.pending_query_vars, &state, &self.program))),
            None => {
                if self.pending_queue.as_ref().map_or(true, |q| q.is_empty()) {
                    self.pending_queue = None;
                    Ok(None)
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub fn query_next(&mut self) -> Option<String> {
        let queue = self.pending_queue.take()?;

        if queue.is_empty() {
            return None;
        }

        let mut solver = Solver::new(&mut self.program);
        let (solution, remaining_queue) = solver.step_until_solution(queue, self.max_steps);

        if remaining_queue.is_empty() {
            self.pending_queue = None;
        } else {
            self.pending_queue = Some(remaining_queue);
        }

        solution.map(|state| format_solution(&self.pending_query_vars, &state, &self.program))
    }

    pub fn has_more_solutions(&self) -> bool {
        self.pending_queue.as_ref().map_or(false, |q| !q.is_empty())
    }

    pub fn query_stop(&mut self) {
        self.pending_queue = None;
        self.pending_query_vars.clear();
    }

    pub fn run_stage(&mut self, stage_index: usize) -> Result<(), String> {
        if stage_index >= self.program.stages.len() {
            return Err(format!("Stage index {} out of bounds", stage_index));
        }

        let stage = &self.program.stages[stage_index];
        if stage.state_constraints.is_empty() {
            return Ok(());
        }

        let constraints = stage.state_constraints.clone();
        let stage_name = stage.name.clone();
        let next_var_map = stage.next_var_map.clone();

        let resolved_state_values: Vec<(String, ir::TermId)> = {
            let true_prop = self.program.props.alloc(ir::Prop::True);
            let mut solver = Solver::new(&mut self.program);
            let solutions: Vec<_> = solver.query_with_strategy(true_prop, self.strategy)
                .with_limit(1)
                .with_max_steps(1000)
                .collect();
            
            if let Some(solution) = solutions.first() {
                self.program.state_vars.clone().into_iter().filter_map(|name| {
                    let term_id = self.var_map.get(&name)?;
                    let resolved = solution.subst.walk(*term_id, &self.program.terms);
                    Some((name, resolved))
                }).collect()
            } else {
                Vec::new()
            }
        };

        let mut all_constraints = Vec::new();
        for (name, resolved_val) in &resolved_state_values {
            if let Some(&original_term) = self.program.state_var_term_ids.get(name) {
                let eq_prop = self.program.props.alloc(ir::Prop::Eq(original_term, *resolved_val));
                all_constraints.push(eq_prop);
            }
        }
        all_constraints.extend(constraints.iter().copied());
        let combined_goal = self.conjoin_props(&all_constraints);

        let mut solver = Solver::new(&mut self.program);
        let state = State::new(combined_goal);
        let solutions: Vec<_> = solver.query_from_state_with_strategy(state, self.strategy)
            .with_limit(2)
            .with_max_steps(self.max_steps)
            .collect();

        match solutions.len() {
            0 => Err(format!(
                "State constraint failure in stage '{}': no solutions found",
                stage_name
            )),
            1 => {
                let solution = &solutions[0];
                for (name, next_term_id) in &next_var_map {
                    let new_value = solution.subst.walk(*next_term_id, &solver.program.terms);
                    self.var_map.insert(name.clone(), new_value);
                }
                Ok(())
            }
            _ => {
                let mut diff_vars = Vec::new();
                for (name, next_term_id) in &next_var_map {
                    let val1 = solver::reify_term(
                        solutions[0].subst.walk(*next_term_id, &solver.program.terms),
                        &solutions[0].subst,
                        solver.program,
                    );
                    let val2 = solver::reify_term(
                        solutions[1].subst.walk(*next_term_id, &solver.program.terms),
                        &solutions[1].subst,
                        solver.program,
                    );
                    if val1 != val2 {
                        diff_vars.push(format!("{}: {} vs {}", name, val1, val2));
                    }
                }
                Err(format!(
                    "Ambiguous state update in stage '{}': multiple solutions found. Differing state vars: [{}]",
                    stage_name,
                    diff_vars.join(", ")
                ))
            }
        }
    }

    pub fn run_stage_by_name(&mut self, stage_name: &str) -> Result<(), String> {
        let stage_index = self.program.stages
            .iter()
            .position(|s| s.name == stage_name)
            .ok_or_else(|| format!("Stage '{}' not found", stage_name))?;
        self.run_stage(stage_index)
    }

    fn conjoin_props(&mut self, props: &[ir::PropId]) -> ir::PropId {
        if props.is_empty() {
            return self.program.props.alloc(ir::Prop::True);
        }
        if props.len() == 1 {
            return props[0];
        }
        let mut result = props[0];
        for &p in &props[1..] {
            result = self.program.props.alloc(ir::Prop::And(result, p));
        }
        result
    }

    pub fn get_state_var(&mut self, name: &str) -> Option<String> {
        let term_id = *self.var_map.get(name)?;
        
        let true_prop = self.program.props.alloc(ir::Prop::True);
        let mut solver = Solver::new(&mut self.program);
        let solutions: Vec<_> = solver.query_with_strategy(true_prop, self.strategy)
            .with_limit(1)
            .with_max_steps(self.max_steps)
            .collect();
        
        if let Some(solution) = solutions.first() {
            Some(solver::reify_term(term_id, &solution.subst, solver.program))
        } else {
            Some(solver::reify_term(term_id, &Subst::new(), &self.program))
        }
    }

    pub fn state_vars(&mut self) -> Vec<(String, String)> {
        let true_prop = self.program.props.alloc(ir::Prop::True);
        let state_var_names = self.program.state_vars.clone();
        let var_map_snapshot: Vec<(String, ir::TermId)> = state_var_names
            .iter()
            .filter_map(|name| {
                self.var_map.get(name).map(|&tid| (name.clone(), tid))
            })
            .collect();
        
        let mut solver = Solver::new(&mut self.program);
        let solutions: Vec<_> = solver.query_with_strategy(true_prop, self.strategy)
            .with_limit(1)
            .with_max_steps(self.max_steps)
            .collect();
        
        let subst = solutions.first().map(|s| &s.subst);
        
        var_map_snapshot
            .into_iter()
            .map(|(name, term_id)| {
                let value = if let Some(s) = subst {
                    solver::reify_term(term_id, s, solver.program)
                } else {
                    solver::reify_term(term_id, &Subst::new(), solver.program)
                };
                (name, value)
            })
            .collect()
    }
}

impl Default for Frontend {
    fn default() -> Self {
        Self::new()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn create_frontend() -> *mut Frontend {
    Box::leak(Box::new(Frontend::new()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_frontend(frontend: *mut Frontend) {
    unsafe { std::ptr::drop_in_place(frontend) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_set_strategy(frontend: *mut Frontend, strategy: i32) {
    unsafe {
        (*frontend).strategy = match strategy {
            0 => SearchStrategy::BFS,
            1 => SearchStrategy::DFS,
            _ => SearchStrategy::BFS,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_get_strategy(frontend: *mut Frontend) -> i32 {
    unsafe {
        match (*frontend).strategy {
            SearchStrategy::BFS => 0,
            SearchStrategy::DFS => 1,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_set_max_steps(frontend: *mut Frontend, max_steps: i32) {
    unsafe {
        (*frontend).max_steps = max_steps as usize;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_get_max_steps(frontend: *mut Frontend) -> i32 {
    unsafe {
        (*frontend).max_steps as i32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_load(frontend: *mut Frontend, source: *const c_char) -> i32 {
    unsafe {
        let source_str = CStr::from_ptr(source).to_str().unwrap_or("");
        match (*frontend).load(source_str) {
            Ok(()) => 0,
            Err(_) => 1,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_query(frontend: *mut Frontend, query: *const c_char) -> *mut c_char {
    unsafe {
        let query_str = CStr::from_ptr(query).to_str().unwrap_or("");
        let result = (*frontend).query(query_str);
        let output = match result {
            Ok(solutions) => {
                if solutions.is_empty() {
                    "no".to_string()
                } else {
                    solutions.join("\n")
                }
            }
            Err(e) => format!("Error: {}", e),
        };
        CString::new(output).unwrap().into_raw()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_query_start(frontend: *mut Frontend, query: *const c_char) -> *mut c_char {
    unsafe {
        let query_str = CStr::from_ptr(query).to_str().unwrap_or("");
        let result = (*frontend).query_start(query_str);
        let output = match result {
            Ok(Some(solution)) => solution,
            Ok(None) => "no".to_string(),
            Err(e) => format!("Error: {}", e),
        };
        CString::new(output).unwrap().into_raw()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_query_next(frontend: *mut Frontend) -> *mut c_char {
    unsafe {
        let result = (*frontend).query_next();
        let output = match result {
            Some(solution) => solution,
            None => "no".to_string(),
        };
        CString::new(output).unwrap().into_raw()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_has_more(frontend: *mut Frontend) -> i32 {
    unsafe {
        if (*frontend).has_more_solutions() { 1 } else { 0 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_query_stop(frontend: *mut Frontend) {
    unsafe {
        (*frontend).query_stop();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_fact_count(frontend: *mut Frontend) -> i32 {
    unsafe { (*frontend).program.facts.len() as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_rule_count(frontend: *mut Frontend) -> i32 {
    unsafe { (*frontend).program.global_rules.len() as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_stage_count(frontend: *mut Frontend) -> i32 {
    unsafe { (*frontend).program.stages.len() as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_stage_name(frontend: *mut Frontend, index: i32) -> *mut c_char {
    unsafe {
        if let Some(stage) = (&(*frontend).program.stages).get(index as usize) {
            CString::new(stage.name.clone()).unwrap().into_raw()
        } else {
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_run_stage(frontend: *mut Frontend, stage_index: i32) -> i32 {
    unsafe {
        match (*frontend).run_stage(stage_index as usize) {
            Ok(()) => 1,
            Err(_) => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_run_stage_by_name(frontend: *mut Frontend, name: *const c_char) -> i32 {
    unsafe {
        let name_str = CStr::from_ptr(name).to_str().unwrap_or("");
        match (*frontend).run_stage_by_name(name_str) {
            Ok(()) => 1,
            Err(_) => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_get_state_var(frontend: *mut Frontend, name: *const c_char) -> *mut c_char {
    unsafe {
        let name_str = CStr::from_ptr(name).to_str().unwrap_or("");
        let value = (*frontend).get_state_var(name_str).unwrap_or_default();
        CString::new(value).unwrap().into_raw()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_state_var_count(frontend: *mut Frontend) -> i32 {
    unsafe { (*frontend).program.state_vars.len() as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_state_var_name(frontend: *mut Frontend, index: i32) -> *mut c_char {
    unsafe {
        if let Some(name) = (&(*frontend).program.state_vars).get(index as usize) {
            CString::new(name.clone()).unwrap().into_raw()
        } else {
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn frontend_state_var_value(frontend: *mut Frontend, index: i32) -> *mut c_char {
    unsafe {
        if let Some(name) = (&(*frontend).program.state_vars).get(index as usize).cloned() {
            let value = (*frontend).get_state_var(&name).unwrap_or_default();
            CString::new(value).unwrap().into_raw()
        } else {
            CString::new("").unwrap().into_raw()
        }
    }
}

fn main() {}

#[cfg(test)]
mod frontend_tests {
    use super::*;

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
