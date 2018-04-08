macro_rules! err {
    ($msg:expr) => {
        TSSParseError {
            msg: $msg.to_owned(),
        }
    };
}

macro_rules! eat {
    ($tokens:expr, $err:expr) => {
        $tokens.next()
            .ok_or(err!($err))
    };


    ($tokens:expr, $rule:ident, $err:expr) => {
        $tokens.next()
            .ok_or(err!($err))
            .and_then(|val| {
                if Rule::$rule != val.as_rule() {
                    Err(err!(&format!("Type is not {:?}", $rule)))
                } else {
                    Ok(val)
                }
            })
    };

    ($tokens:expr, [$( $rule:ident ),+], $err:expr) => {
        $tokens.next()
            .ok_or(err!($err))
            .and_then(|val| {
                $(
                    if Rule::$rule == val.as_rule() {
                        return Ok(val);
                    }
                )*
                return Err(err!("Type is wrong"))
            })
    };
}

macro_rules! to_idents {
    ($ident_list: expr) => {
        $ident_list?.into_inner()
            .map(|id| id.as_str())
            .map(String::from)
            .collect()
    };
}

use ast::AST;
use grammar::{Rule, TensorScriptParser};
use pest::iterators::Pair;
use pest::Parser;

#[derive(Debug)]
pub struct TSSParseError {
    msg: String,
}

use grammar::Rule::*;

pub fn parse_str(source: &str) -> Result<AST, TSSParseError> {
    let parser = TensorScriptParser::parse(Rule::input, source);
    if parser.is_err() {
        panic!(format!("{:#}", parser.err().unwrap()));
    }

    let top_levels = parser.unwrap().map(|pair| consume(pair).unwrap()).collect();
    Ok(AST::List(top_levels))
}

pub fn consume(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    // println!("{}", pair);
    match pair.as_rule() {
        use_stmt => build_use_stmt(pair),
        node_decl => build_node_decl(pair),
        fn_type_sig => build_fn_type_sig(pair),
        node_decl_body => build_node_decl_body(pair),
        node_macro_assign => build_node_macro_assign(pair),
        int_lit => build_int_lit(pair),
        float_lit => build_float_lit(pair),
        weights_decl => build_weights_decl(pair),
        weights_decl_body => build_weights_decl_body(pair),
        weights_assign => build_weights_assign(pair),
        fn_call => build_fn_call(pair),
        fn_call_args => build_fn_call_args(pair),
        fn_call_arg => build_fn_call_arg(pair),
        graph_decl => build_graph_decl(pair),
        graph_decl_body => build_graph_decl_body(pair),

        fn_decls => build_fn_decls(pair),
        fn_decl_args => build_fn_decl_args(pair),
        fn_decl_arg => build_fn_decl_arg(pair),
        fn_decl => build_fn_decl(pair),
        fn_decl_param => build_fn_decl_param(pair),
        fn_decl_sig => build_fn_decl_sig(pair),
        stmts => build_stmts(pair),

        stmt => build_stmt(pair),
        expr => build_expr(pair),
        field_access => build_field_access(pair),
        fn_call_param => build_fn_call_param(pair),
        block => build_block(pair),
        pipes => build_pipes(pair),

        // Rule::statements                  => build_block(pair),
        // Rule::integer_zero_literal        => integer!(0),
        // Rule::integer_binary_literal      => build_integer(pair, 2),
        // Rule::integer_octal_literal       => build_integer(pair, 8),
        // Rule::integer_decimal_literal     => build_integer(pair, 10),
        // Rule::integer_hexadecimal_literal => build_integer(pair, 16),
        // Rule::float_literal               => build_float(pair),
        // Rule::atom_literal                => build_atom(pair),
        // Rule::string_literal              => build_string(pair),
        // Rule::bool_true                   => boolean!(true),
        // Rule::bool_false                  => boolean!(false),
        // Rule::unary_not                   => unary_not!(consume(pair.into_inner().next().unwrap())),
        // Rule::unary_complement            => unary_complement!(consume(pair.into_inner().next().unwrap())),
        // Rule::unary_plus                  => unary_plus!(consume(pair.into_inner().next().unwrap())),
        // Rule::unary_minus                 => unary_minus!(consume(pair.into_inner().next().unwrap())),
        // Rule::braced_expression           => braced!(consume(pair.into_inner().next().unwrap())),
        // Rule::const_literal               => build_const(pair),
        // Rule::local_var_access            => build_lvar_access(pair),
        // Rule::call_with_implicit_receiver => build_implicit_call(pair.into_inner().next().unwrap()),
        // Rule::call_with_explicit_receiver => build_explicit_call(pair),
        // Rule::list_literal                => build_list(pair),
        // Rule::index                       => build_index(pair),
        // Rule::function_literal            => build_function(pair),
        // Rule::block                       => build_block(pair),
        // Rule::map_literal                 => build_map(pair),
        _ => unexpected_token(pair),
    }
}

fn build_block(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let statements = eat!(tokens, stmts, "Cannot parse statements");
    let possible_expr = eat!(tokens, expr, "Does not have a dangling expr");
    let ret = if possible_expr.is_err() {
        AST::None
    } else {
        consume(possible_expr?)?
    };

    Ok(AST::Block {
        stmts: Box::new(consume(statements?)?),
        ret: Box::new(ret),
    })
}

fn build_fn_call_param(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let args = eat!(tokens, fn_call_args, "Does not have args");
    if args.is_err() {
        Ok(AST::List(vec![]))
    } else {
        consume(args?)
    }
}

fn build_field_access(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let var_name = eat!(tokens, ident, "Failed to parse variable name")?;
    let field_name = eat!(tokens, ident, "Failed to parse field name")?;
    let func_call = eat!(tokens, fn_call_param, "Is not a function call");
    let func_call = if func_call.is_err() {
        AST::None
    } else {
        consume(func_call?)?
    };

    Ok(AST::FieldAccess {
        var_name: var_name.as_str().to_owned(),
        field_name: field_name.as_str().to_owned(),
        func_call: Box::new(func_call),
    })
}

fn build_expr(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let tokens = pair.into_inner();
    let vals = tokens.map(|p| consume(p).unwrap()).collect();
    Ok(AST::Expr {
        items: Box::new(AST::List(vals)),
    })
}

fn build_stmt(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let tokens = pair.into_inner();
    let vals = tokens.map(|p| consume(p).unwrap()).collect();
    Ok(AST::Stmt {
        items: Box::new(AST::List(vals)),
    })
}

fn build_fn_decl_param(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let tokens = pair.into_inner();
    let vals = tokens.map(|p| consume(p).unwrap()).collect();
    Ok(AST::List(vals))
}

fn build_fn_decl(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let mut head = eat!(tokens, fn_decl_head, "Failed to parse fn_decl_head")?.into_inner();
    let name = eat!(head, ident, "Failed to parse fn_decl_head ident")?;
    let fn_sig = eat!(head, fn_decl_sig, "Failed to parse fn decl signature")?;
    let func_block = eat!(tokens, block, "Failed to parse function block")?;

    let mut tokens = fn_sig.into_inner();
    let param = eat!(tokens, fn_decl_param, "Failed to parse fn_decl_param")?;
    let return_type = {
        let temp = eat!(tokens, type_sig, "Function does not have a type signature");
        if temp.is_err() {
            vec![]
        } else {
            to_idents!(temp)
        }
    };

    Ok(AST::FnDecl {
        name: name.as_str().to_owned(),
        fn_params: Box::new(consume(param)?),
        return_type: return_type,
        func_block: Box::new(consume(func_block)?),
    })
}

fn build_fn_decl_sig(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    unimplemented!()
}

fn build_stmts(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let tokens = pair.into_inner();
    let vals = tokens.map(|p| consume(p).unwrap()).collect();
    Ok(AST::List(vals))
}

fn build_fn_decls(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let tokens = pair.into_inner();
    let vals = tokens.map(|p| consume(p).unwrap()).collect();
    Ok(AST::List(vals))
}

fn build_fn_decl_arg(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let param = eat!(tokens, ident, "Failed to parse function parameter")?;
    let typ = eat!(tokens, type_sig, "Failed to parse type signature");
    let typ = if typ.is_err() {
        vec![]
    } else {
        to_idents!(typ)
    };

    Ok(AST::FnDeclArg {
        name: param.as_str().to_owned(),
        type_sig: typ,
    })
}

fn build_fn_decl_args(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let tokens = pair.into_inner();
    let vals = tokens.map(|p| consume(p).unwrap()).collect();
    Ok(AST::List(vals))
}

fn build_fn_call(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let name = eat!(tokens, ident, "Cannot parse function call identifier")?;
    let args = if let Some(args) = tokens.next() {
        consume(args)?
    } else {
        AST::List(vec![])
    };

    Ok(AST::FnCall {
        name: name.as_str().to_owned(),
        args: Box::new(args),
    })
}

fn build_fn_call_arg(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let param = eat!(tokens, ident, "Failed to parse function call argument")?;
    let param_val = eat!(tokens, expr, "Failed to parse function call parameter")?;

    Ok(AST::FnCallArg {
        name: param.as_str().to_owned(),
        arg: Box::new(consume(param_val)?),
    })
}

fn build_fn_call_args(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let tokens = pair.into_inner();
    let vals = tokens.map(|p| consume(p).unwrap()).collect();

    Ok(AST::List(vals))
}

fn build_weights_decl(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let mut head = eat!(tokens, weights_decl_head, "Parsing `weight_head` error")?.into_inner();
    let weights_name = eat!(head, cap_ident, "Does not have a weight name")?.as_str();
    let type_decl = eat!(head, fn_type_sig, "Failed to parse `fn_type_sig`")?;
    let weights_body = eat!(
        tokens,
        weights_decl_body,
        "Failed to parse `weights_decl_body`"
    )?;

    Ok(AST::WeightsDecl {
        name: weights_name.to_owned(),
        type_sig: Box::new(consume(type_decl)?),
        initialization: Box::new(consume(weights_body)?),
    })
}

fn build_weights_decl_body(body: Pair<Rule>) -> Result<AST, TSSParseError> {
    let tokens = body.into_inner();
    let vals = tokens.map(|p| consume(p).unwrap()).collect();

    Ok(AST::List(vals))
}

fn build_weights_assign(body: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = body.into_inner();
    let name = eat!(tokens, ident, "Failed to parse ident")?;
    let _assign = eat!(tokens, op_assign, "Failed to parse `=`")?;
    let mod_name = eat!(tokens, cap_ident, "Failed to parse `mod_name`")?;
    let fn_sig = eat!(tokens, fn_type_sig, "Failed to parse `fn_sig`")?;
    let func = eat!(tokens, fn_call, "Failed to parse `fn_call`")?;

    Ok(AST::WeightsAssign {
        name: name.as_str().to_owned(),
        mod_name: mod_name.as_str().to_owned(),
        mod_sig: Box::new(consume(fn_sig)?),
        func: Box::new(consume(func)?),
    })
}

fn _process_level(curr: Pair<Rule>) -> AST {
    if curr.as_rule() == fn_call || curr.as_rule() == field_access {
        consume(curr).unwrap()
    } else if curr.as_rule() == ident {
        AST::Ident(curr.as_str().to_owned())
    } else {
        println!("{:?}", curr.as_rule());
        unimplemented!()
    }
}

fn build_pipes(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    // linearizes from tree
    let mut exprs = vec![];
    let mut tokens = pair.into_inner(); // [ident, expr]
    loop {
        let curr = tokens.next();
        if curr.is_none() { break; }
        let curr = curr.unwrap();
        if curr.as_rule() == expr { // { expr { pipe | ident | fn_call } }
            let temp = curr.into_inner().next();
            if temp.is_none() { break; }
            let temp = temp.unwrap();
            if temp.as_rule() == pipes {
                tokens = temp.into_inner(); // { pipe | ident | fn_call }
                continue;
            }
            exprs.push(_process_level(temp.clone()));
        } else {
            exprs.push(_process_level(curr).clone());
        }
    }

    // construct a deep fn_call recursively
    let mut iter = exprs.iter();
    let mut init = iter.next().unwrap().to_owned();

    while let Some(node) = iter.next() {
        init = match node {
            &AST::Ident(ref name) => AST::FnCall {
                name: name.clone(),
                args: Box::new(AST::List(vec![
                    AST::FnCallArg {
                        name: format!("x"),
                        arg: Box::new(init),
                    }
                ]))
            },
            &AST::FnCall{ref name, ref args} => AST::FnCall {
                name: name.clone(),
                args: AST::extend_arg_list(args.clone(), init),
            },
            _ => {
                println!("{:?}", node);
                unimplemented!()
            }
        };
    }

    Ok(init)
}

fn build_graph_decl(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let mut head = eat!(tokens, graph_decl_head, "Parsing `graph_head` error")?.into_inner();
    let node_name = eat!(head, cap_ident, "Does not have a graph name")?.as_str();
    let type_decl = eat!(head, fn_type_sig, "Failed to parse `fn_type_sig`")?;
    let graph_body = eat!(tokens, graph_decl_body, "Failed to parse `graph_decl_body`")?;

    Ok(AST::GraphDecl {
        name: node_name.to_owned(),
        type_sig: Box::new(consume(type_decl)?),
        fns: Box::new(consume(graph_body)?),
    })
}

fn build_graph_decl_body(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let fns = eat!(tokens, fn_decls, "Failed to parse `fn_decls`")?;
    let vals = fns.into_inner().map(|p| consume(p).unwrap()).collect();
    Ok(AST::List(vals))
}

fn build_node_decl(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let mut head = eat!(tokens, node_decl_head, "Parsing `node_head` error")?.into_inner();
    let node_name = eat!(head, cap_ident, "Does not have a node name")?.as_str();
    let type_decl = eat!(head, fn_type_sig, "Failed to parse `fn_type_sig`")?;
    let node_body = eat!(tokens, node_decl_body, "Failed to parse `node_decl_body`")?;

    Ok(AST::NodeDecl {
        name: node_name.to_owned(),
        type_sig: Box::new(consume(type_decl)?),
        initialization: Box::new(consume(node_body)?),
    })
}

fn build_node_decl_body(body: Pair<Rule>) -> Result<AST, TSSParseError> {
    let tokens = body.into_inner();
    let vals = tokens.map(|p| consume(p).unwrap()).collect();

    Ok(AST::List(vals))
}

fn build_node_macro_assign(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    if pair.as_rule() != node_macro_assign {
        return Err(err!(format!("Type mismatch: {:?}", node_macro_assign)));
    }
    let mut tokens = pair.into_inner();
    let identifier = eat!(tokens, upper_ident, "Failed to parse `upper_ident`")?;
    let _assign = eat!(tokens, op_assign, "Cannot parse `=`")?;
    let lit = eat!(tokens, [int_lit, float_lit], "Cannot parse literal")?;

    let identifier = identifier.as_str().to_owned();
    let lit = consume(lit)?;

    Ok(AST::MacroAssign(identifier, Box::new(lit)))
}

fn build_float_lit(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let ret = pair.as_str().parse().unwrap();
    Ok(AST::Float(ret))
}

fn build_int_lit(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let ret = pair.as_str().parse().unwrap();
    Ok(AST::Integer(ret))
}

fn build_fn_type_sig(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let ident_list_from = eat!(tokens, type_ident_list, "Cannot parse type_ident_list");
    let from_type = to_idents!(ident_list_from);
    let ident_list_to = eat!(tokens, type_ident_list, "Cannot parse type_ident_list");
    let to_type = to_idents!(ident_list_to);

    Ok(AST::FnTypeSig(from_type, to_type))
}

fn build_use_stmt(pair: Pair<Rule>) -> Result<AST, TSSParseError> {
    let mut tokens = pair.into_inner();
    let value = eat!(tokens, use_lit, "Parsing `use` error")?;
    let module_name = eat!(tokens, ident, "module name not defined")?.as_str();
    let imported = eat!(tokens, "no imported modules")?;

    let mut imported_tokens = vec![];
    match imported.as_rule() {
        Rule::ident_list => imported
            .into_inner()
            .map(|tok| imported_tokens.push(tok.as_str().to_owned()))
            .collect(),
        Rule::ident => imported_tokens.push(imported.as_str().to_owned()),
        _ => unexpected_token(imported),
    };

    Ok(AST::UseStmt {
        mod_name: module_name.to_owned(),
        imported_names: imported_tokens,
    })
}

fn unexpected_token(pair: Pair<Rule>) -> ! {
    let message = format!("Unexpected token: {:#}", pair);
    panic!(message);
}
