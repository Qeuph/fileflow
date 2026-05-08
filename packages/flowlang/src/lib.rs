use pest_derive::Parser;
use pest::Parser;
use anyhow::{anyhow, Result};

#[derive(Parser)]
#[grammar = "flowlang.pest"]
pub struct FlowLangParser;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum ASTNode {
    Pipeline { name: String, body: Vec<ASTNode> },
    Let { name: String, value: Box<ASTNode> },
    For { var: String, iterable: Box<ASTNode>, body: Vec<ASTNode> },
    Call { name: String, args: Vec<(Option<String>, ASTNode)> },
    Pipe { nodes: Vec<ASTNode> },
    Ident(String),
    String(String),
    Number(i64),
}

pub fn parse(input: &str) -> Result<ASTNode> {
    let mut pairs = FlowLangParser::parse(Rule::program, input)?;
    let program_pair = pairs.next().ok_or_else(|| anyhow!("Empty program"))?;
    
    for pair in program_pair.into_inner() {
        match pair.as_rule() {
            Rule::pipeline => return parse_pipeline(pair),
            Rule::EOI => (),
            _ => unreachable!(),
        }
    }
    
    Err(anyhow!("No pipeline found"))
}

fn parse_pipeline(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode> {
    let mut inner = pair.into_inner();
    let name_pair = inner.next().ok_or_else(|| anyhow!("Pipeline missing name"))?;
    let name = name_pair.as_str().trim_matches('\"').to_string();
    
    let mut body = Vec::new();
    for stmt_pair in inner {
        body.push(parse_statement(stmt_pair)?);
    }
    
    Ok(ASTNode::Pipeline { name, body })
}

fn parse_statement(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode> {
    let inner = pair.into_inner().next().ok_or_else(|| anyhow!("Empty statement"))?;
    match inner.as_rule() {
        Rule::let_stmt => parse_let(inner),
        Rule::for_stmt => parse_for(inner),
        Rule::call_stmt => parse_call_chain(inner),
        _ => unreachable!(),
    }
}

fn parse_let(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let value = parse_expression(inner.next().unwrap())?;
    Ok(ASTNode::Let { name, value: Box::new(value) })
}

fn parse_for(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode> {
    let mut inner = pair.into_inner();
    let var = inner.next().unwrap().as_str().to_string();
    let iterable = parse_expression(inner.next().unwrap())?;
    let mut body = Vec::new();
    for stmt in inner {
        body.push(parse_statement(stmt)?);
    }
    Ok(ASTNode::For { var, iterable: Box::new(iterable), body })
}

fn parse_call_chain(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode> {
    let mut inner = pair.into_inner();
    let mut nodes = Vec::new();
    
    while let Some(ident_pair) = inner.next() {
        let name = ident_pair.as_str().to_string();
        let mut args = Vec::new();
        if let Some(arg_list_pair) = inner.next() {
            if arg_list_pair.as_rule() == Rule::arg_list {
                args = parse_arg_list(arg_list_pair)?;
            }
        }
        nodes.push(ASTNode::Call { name, args });
    }
    
    if nodes.len() == 1 {
        Ok(nodes.remove(0))
    } else {
        Ok(ASTNode::Pipe { nodes })
    }
}

fn parse_expression(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode> {
    let inner = pair.into_inner().next().ok_or_else(|| anyhow!("Empty expression"))?;
    match inner.as_rule() {
        Rule::call_expr => {
            let mut inner = inner.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let mut args = Vec::new();
            if let Some(arg_list) = inner.next() {
                args = parse_arg_list(arg_list)?;
            }
            Ok(ASTNode::Call { name, args })
        }
        Rule::ident => Ok(ASTNode::Ident(inner.as_str().to_string())),
        Rule::string => Ok(ASTNode::String(inner.as_str().trim_matches('\"').to_string())),
        Rule::number => Ok(ASTNode::Number(inner.as_str().parse()?)),
        _ => unreachable!(),
    }
}

fn parse_arg_list(pair: pest::iterators::Pair<Rule>) -> Result<Vec<(Option<String>, ASTNode)>> {
    let mut args = Vec::new();
    for arg_pair in pair.into_inner() {
        let mut inner = arg_pair.into_inner();
        let first = inner.next().unwrap();
        if let Some(second) = inner.next() {
            // Named arg: ident = expression
            args.push((Some(first.as_str().to_string()), parse_expression(second)?));
        } else {
            // Positional arg: expression
            args.push((None, parse_expression(first)?));
        }
    }
    Ok(args)
}
