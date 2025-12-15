pub mod parser;

#[cfg(test)]
mod parser_tests;

use std::fmt;

type Span<'a> = nom_locate::LocatedSpan<&'a str>;

#[derive(Debug)]
pub struct Module<'a> {
    pub span: Span<'a>,
    pub facts: Vec<Term<'a>>,
    pub global_stage: Stage<'a>,
    pub stages: Vec<Stage<'a>>,
}

#[derive(Debug)]
pub struct Stage<'a> {
    pub span: Span<'a>,
    pub name: &'a str,
    pub rules: Vec<Rule<'a>>,
}

#[derive(Debug)]
pub struct Rule<'a> {
    pub span: Span<'a>,
    pub name: &'a str,
    pub premise: Term<'a>,
    pub conclusion: Term<'a>,
}

#[derive(Debug)]
pub struct Term<'a> {
    pub span: Span<'a>,
    pub contents: TermContents<'a>,
}

#[derive(Debug)]
pub enum TermContents<'a> {
    App { rel: Rel<'a>, args: Vec<Term<'a>> },
    Atom { text: &'a str },
    Var { name: &'a str },
    Int { val: i32 },
    Float { val: f32 },
}

#[derive(Debug)]
pub enum Rel<'a> {
    SMTRel { name: &'a str },
    UserRel { name: &'a str },
}

impl<'a> fmt::Display for Term<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.contents {
            TermContents::App { rel, args } => {
                let rel_name = match rel {
                    Rel::SMTRel { name } => name,
                    Rel::UserRel { name } => name,
                };
                write!(f, "{}", rel_name)?;
                if !args.is_empty() {
                    write!(f, "(")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            TermContents::Atom { text } => write!(f, "{}", text),
            TermContents::Var { name } => write!(f, "{}", name),
            TermContents::Int { val } => write!(f, "{}", val),
            TermContents::Float { val } => write!(f, "{}", val),
        }
    }
}

impl<'a> fmt::Display for Rule<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Rule {}:", self.name)?;

        let premise_str = format!("{}", self.premise);
        let conclusion_str = format!("{}", self.conclusion);
        let max_len = premise_str.len().max(conclusion_str.len());
        let dashes = "-".repeat(max_len);

        writeln!(f, "    {}", premise_str)?;
        writeln!(f, "    {}", dashes)?;
        writeln!(f, "    {}", conclusion_str)
    }
}

impl<'a> fmt::Display for Stage<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Begin Stage {}:", self.name)?;
        for rule in &self.rules {
            write!(f, "{}", rule)?;
        }
        writeln!(f, "End Stage {}", self.name)
    }
}

impl<'a> fmt::Display for Module<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print Facts section
        writeln!(f, "Begin Facts:")?;
        for fact in &self.facts {
            writeln!(f, "    {}", fact)?;
        }
        writeln!(f, "End Facts")?;
        writeln!(f)?;

        // Print Global section
        writeln!(f, "Begin Global:")?;
        for rule in &self.global_stage.rules {
            write!(f, "{}", rule)?;
        }
        writeln!(f, "End Global")?;

        // Print regular stages
        for stage in &self.stages {
            writeln!(f)?;
            write!(f, "{}", stage)?;
        }
        Ok(())
    }
}
