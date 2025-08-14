use crate::thir::THir;

use std::cell::RefCell;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use baml_types::{BamlMap, BamlValueWithMeta};
use crate::thir::{Expr, ExprMetadata, Block, Statement, VarIndex};

/// A scope is a map of variable names to their values.
/// 
/// Variables are stored in refcells to allow for mutation.
pub struct Scope {
    pub variables: BamlMap<String, RefCell<BamlValueWithMeta<ExprMetadata>>>,
}

enum EvalValue {
    Value(BamlValueWithMeta<ExprMetadata>),
    Function(usize, Arc<Block<ExprMetadata>>, ExprMetadata),
}

#[derive(Debug)]
enum ControlFlow {
    Normal(BamlValueWithMeta<ExprMetadata>),
    Break,
    Continue,
    Return(BamlValueWithMeta<ExprMetadata>),
}

pub fn interpret_thir(
    thir: THir<ExprMetadata>,
    expr: Expr<ExprMetadata>,
) -> Result<BamlValueWithMeta<ExprMetadata>> {
    let mut scopes = vec![Scope { variables: BamlMap::new() }];

    // Seed scope with global assignments
    for (name, gexpr) in thir.global_assignments.iter() {
        let v = expect_value(evaluate_expr(gexpr, &mut scopes)?)?;
        declare(&mut scopes, name, v);
    }

    // Evaluate provided expression
    let result = expect_value(evaluate_expr(&expr, &mut scopes)?)?;
    Ok(result)
}

fn evaluate_block_with_control_flow(block: &Block<ExprMetadata>, scopes: &mut Vec<Scope>) -> Result<ControlFlow> {
    scopes.push(Scope { variables: BamlMap::new() });
    for stmt in block.statements.iter() {
        match stmt {
            Statement::Let { name, value, .. } => {
                let v = expect_value(evaluate_expr(value, scopes)?)?;
                declare(scopes, name, v);
            }
            Statement::Declare { name, span } => {
                declare(scopes, name, BamlValueWithMeta::Null((span.clone(), None)));
            }
            Statement::Assign { name, value } => {
                let v = expect_value(evaluate_expr(value, scopes)?)?;
                assign(scopes, name, v)?;
            }
            Statement::DeclareAndAssign { name, value, .. } => {
                let v = expect_value(evaluate_expr(value, scopes)?)?;
                declare(scopes, name, v);
            }
            Statement::FunctionReturn { expr, .. } => {
                let v = expect_value(evaluate_expr(expr, scopes)?)?;
                scopes.pop();
                return Ok(ControlFlow::Return(v));
            }
            Statement::Expression { expr, .. } => {
                let _ = evaluate_expr(expr, scopes)?;
            }
            Statement::Break(_) => {
                scopes.pop();
                return Ok(ControlFlow::Break);
            }
            Statement::Continue(_) => {
                scopes.pop();
                return Ok(ControlFlow::Continue);
            }
            Statement::While { condition, block, .. } => {
                loop {
                    let cond_val = expect_value(evaluate_expr(condition, scopes)?)?;
                    match cond_val {
                        BamlValueWithMeta::Bool(true, _) => {
                            match evaluate_block_with_control_flow(block, scopes)? {
                                ControlFlow::Break => break,
                                ControlFlow::Continue => continue,
                                ControlFlow::Normal(_) => {},
                                ControlFlow::Return(val) => {
                                    scopes.pop();
                                    return Ok(ControlFlow::Return(val));
                                }
                            }
                        },
                        BamlValueWithMeta::Bool(false, _) => break,
                        _ => bail!("while condition must be boolean"),
                    }
                }
            }
            Statement::ForLoop { identifier, iterator, block, .. } => {
                let iterable_val = expect_value(evaluate_expr(iterator, scopes)?)?;
                match iterable_val {
                    BamlValueWithMeta::List(items, _) => {
                        for item_val in items.iter() {
                            // Create new scope for loop iteration
                            scopes.push(Scope { variables: BamlMap::new() });
                            declare(scopes, identifier, item_val.clone());
                            
                            match evaluate_block_with_control_flow(block, scopes)? {
                                ControlFlow::Break => {
                                    scopes.pop();
                                    break;
                                },
                                ControlFlow::Continue => {
                                    scopes.pop();
                                    continue;
                                },
                                ControlFlow::Normal(_) => {
                                    scopes.pop();
                                },
                                ControlFlow::Return(val) => {
                                    scopes.pop();
                                    scopes.pop();
                                    return Ok(ControlFlow::Return(val));
                                }
                            }
                        }
                    },
                    _ => bail!("for loop requires iterable (list)"),
                }
            }
        }
    }
    let ret = expect_value(evaluate_expr(&block.return_value, scopes)?)?;
    scopes.pop();
    Ok(ControlFlow::Normal(ret))
}

fn evaluate_block(block: &Block<ExprMetadata>, scopes: &mut Vec<Scope>) -> Result<BamlValueWithMeta<ExprMetadata>> {
    match evaluate_block_with_control_flow(block, scopes)? {
        ControlFlow::Normal(val) => Ok(val),
        ControlFlow::Return(val) => Ok(val),
        ControlFlow::Break => bail!("break statement not in loop context"),
        ControlFlow::Continue => bail!("continue statement not in loop context"),
    }
}

fn declare(scopes: &mut Vec<Scope>, name: &str, v: BamlValueWithMeta<ExprMetadata>) {
    if let Some(top) = scopes.last_mut() {
        top.variables.insert(name.to_string(), RefCell::new(v));
    }
}

fn assign(scopes: &mut [Scope], name: &str, v: BamlValueWithMeta<ExprMetadata>) -> Result<()> {
    for s in scopes.iter().rev() {
        if let Some(cell) = s.variables.get(name) {
            *cell
                .try_borrow_mut()
                .map_err(|_| anyhow!("variable `{}` is currently borrowed", name))? = v;
            return Ok(());
        }
    }
    bail!("assign to undeclared variable `{}`", name)
}

fn lookup(scopes: &[Scope], name: &str) -> Option<BamlValueWithMeta<ExprMetadata>> {
    for s in scopes.iter().rev() {
        if let Some(cell) = s.variables.get(name) {
            return Some(cell.borrow().clone());
        }
    }
    None
}

fn evaluate_expr(expr: &Expr<ExprMetadata>, scopes: &mut Vec<Scope>) -> Result<EvalValue> {
    Ok(match expr {
        Expr::Atom(v) => EvalValue::Value(v.clone()),
        Expr::List(items, meta) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items.iter() {
                out.push(expect_value(evaluate_expr(it, scopes)?)?);
            }
            EvalValue::Value(BamlValueWithMeta::List(out, meta.clone()))
        }
        Expr::Map(entries, meta) => {
            let mut out: BamlMap<String, BamlValueWithMeta<ExprMetadata>> = BamlMap::new();
            for (k, v) in entries.iter() {
                out.insert(k.clone(), expect_value(evaluate_expr(v, scopes)?)?);
            }
            EvalValue::Value(BamlValueWithMeta::Map(out, meta.clone()))
        }
        Expr::Block(block, _meta) => {
            let v = evaluate_block(block, scopes)?;
            EvalValue::Value(v)
        }
        Expr::FreeVar(name, meta) => {
            let v = lookup(scopes, name).with_context(|| format!("unbound variable `{}` at {:?}", name, meta.0))?;
            EvalValue::Value(v)
        }
        Expr::BoundVar(_, _) => bail!("unexpected bound var outside func application"),
        Expr::Function(arity, body, meta) => EvalValue::Function(*arity, body.clone(), meta.clone()),
        Expr::Call { func, type_args: _, args, meta: _ } => {
            let callee = evaluate_expr(func, scopes)?;
            let (arity, body, meta) = match callee { EvalValue::Function(a, b, m) => (a, b, m), _ => bail!("attempted to call non-function") };
            if arity != args.len() { bail!("arity mismatch: expected {} args, got {}", arity, args.len()); }

            // Evaluate arguments first
            let mut arg_vals: Vec<BamlValueWithMeta<ExprMetadata>> = Vec::with_capacity(args.len());
            for a in args.iter() { arg_vals.push(expect_value(evaluate_expr(a, scopes)?)?); }

            // Create fresh names and open body under them
            let body_expr = Expr::Block(Box::new(Arc::unwrap_or_clone(body.clone())), meta.clone());
            let fresh = body_expr.fresh_names(arity);
            let mut opened = body_expr;
            for (i, name) in fresh.iter().enumerate() {
                opened = opened.open(&VarIndex{ de_bruijn: 0, tuple: i as u32 }, name);
            }

            // Create a scope binding parameters to their argument values
            scopes.push(Scope{ variables: fresh.into_iter().zip(arg_vals.into_iter()).map(|(k,v)| (k, RefCell::new(v))).collect() });
            let result = match &opened {
                Expr::Block(b, _) => evaluate_block(b, scopes)?,
                other => expect_value(evaluate_expr(other, scopes)?)?,
            };
            scopes.pop();
            EvalValue::Value(result)
        }
        Expr::If(cond, then, else_, meta) => {
            let cv = expect_value(evaluate_expr(cond, scopes)?)?;
            let b = match cv { BamlValueWithMeta::Bool(v, _) => v, _ => bail!("condition not bool at {:?}", meta.0) };
            if b {
                EvalValue::Value(expect_value(evaluate_expr(then, scopes)?)?)
            } else if let Some(e) = else_ {
                EvalValue::Value(expect_value(evaluate_expr(e, scopes)?)?)
            } else {
                EvalValue::Value(BamlValueWithMeta::Null(meta.clone()))
            }
        }
        Expr::ArrayAccess { base, index, meta } => {
            let b = expect_value(evaluate_expr(base, scopes)?)?;
            let i = expect_value(evaluate_expr(index, scopes)?)?;
            let arr = match b.clone() {
                BamlValueWithMeta::List(v, _) => v,
                _ => bail!("array access on non-list at {:?}", meta)
            };
            let idx = match i { BamlValueWithMeta::Int(ii, _) => ii as usize, _ => bail!("index not int at {:?}", meta) };
            let v = arr.get(idx).cloned().context("index out of bounds")?;
            EvalValue::Value(v.clone())
        }
        Expr::FieldAccess { base, field, meta } => {
            let b = expect_value(evaluate_expr(base, scopes)?)?;
            match b.clone() {
                BamlValueWithMeta::Map(m, _) => {
                    let v = m.get(field).context("missing field")?;
                    EvalValue::Value(v.clone())
                },
                BamlValueWithMeta::Class(_, m, _) => {
                    let v = m.get(field).context("missing field")?;
                    EvalValue::Value(v.clone())
                },
                _ => bail!("field access on non-map/class at {:?}", meta.0),
            }
        }
        Expr::ClassConstructor { name, fields, spread, meta } => {
            let mut field_map: BamlMap<String, BamlValueWithMeta<ExprMetadata>> = BamlMap::new();
            
            // Handle spread first if present
            if let Some(spread_expr) = spread {
                let spread_val = expect_value(evaluate_expr(spread_expr, scopes)?)?;
                match spread_val.clone() {
                    BamlValueWithMeta::Class(_, spread_fields, _) => {
                        for (k, v) in spread_fields.iter() {
                            field_map.insert(k.clone(), v.clone());
                        }
                    }
                    // // TODO: Allow maps to be spread?
                    // BamlValueWithMeta::Map(spread_fields) => {
                    //     for (k, v) in spread_fields.iter() {
                    //         field_map.insert(k.clone(), v.clone());
                    //     }
                    // }
                    _ => bail!("spread operator can only be used on classes at {:?}", meta.0),
                }
            }
            
            // Evaluate and insert explicit fields (these override spread fields)
            for (k, v) in fields.iter() {
                field_map.insert(k.clone(), expect_value(evaluate_expr(v, scopes)?)?);
            }
            
            EvalValue::Value(BamlValueWithMeta::Class(name.clone(), field_map, meta.clone()))
        }
        Expr::Builtin(builtin, meta) => {
            use crate::thir::Builtin;
            match builtin {
                Builtin::FetchValue => {
                    // FetchValue requires network access and is not supported in the interpreter
                    bail!("builtin function std::fetch_value is not supported in interpreter at {:?}", meta.0)
                }
            }
        }
        Expr::BinaryOperation { left, operator, right, meta } => {
            let left_val = expect_value(evaluate_expr(left, scopes)?)?;
            let right_val = expect_value(evaluate_expr(right, scopes)?)?;
            

            let result = evaluate_binary_op(operator, &left_val, &right_val, meta)?;
            EvalValue::Value(result)
        }
        Expr::UnaryOperation { operator, expr, meta } => {
            let val = expect_value(evaluate_expr(expr, scopes)?)?;
            

            let result = evaluate_unary_op(operator, &val, meta)?;
            EvalValue::Value(result)
        }
        Expr::ForLoop { item, iterable, body, meta } => {
            let iterable_val = expect_value(evaluate_expr(iterable, scopes)?)?;
            match iterable_val {
                BamlValueWithMeta::List(items,_) => {
                    let mut results = Vec::with_capacity(items.len());
                    for item_val in items.iter() {
                        // Create new scope for loop iteration
                        scopes.push(Scope { variables: BamlMap::new() });
                        declare(scopes, item, item_val.clone());
                        
                        let result = expect_value(evaluate_expr(body, scopes)?)?;
                        results.push(result);
                        
                        scopes.pop();
                    }
                    EvalValue::Value(BamlValueWithMeta::List(results, meta.clone()))
                },
                _ => bail!("for loop requires iterable (list) at {:?}", meta.0),
            }
        }
    })
}

fn expect_value(v: EvalValue) -> Result<BamlValueWithMeta<ExprMetadata>> {
    match v {
        EvalValue::Value(v) => Ok(v),
        EvalValue::Function(_, _, _) => bail!("expected value, found function"),
    }
}

fn evaluate_binary_op(
    operator: &crate::hir::BinaryOperator,
    left_val: &BamlValueWithMeta<ExprMetadata>,
    right_val: &BamlValueWithMeta<ExprMetadata>,
    meta: &ExprMetadata,
) -> Result<BamlValueWithMeta<ExprMetadata>> {
    use crate::hir::BinaryOperator;
    Ok(match operator {
        // Arithmetic operations
        BinaryOperator::Add => match (left_val.clone(), right_val.clone()) {
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => BamlValueWithMeta::Int(a + b, meta.clone()),
            (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Float(b,_)) => BamlValueWithMeta::Float(a + b, meta.clone()),
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Float(b,_)) => BamlValueWithMeta::Float(a as f64 + b, meta.clone()),
            (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Int(b,_)) => BamlValueWithMeta::Float(a + (b as f64), meta.clone()),
            (BamlValueWithMeta::String(a,_), BamlValueWithMeta::String(b,_)) => BamlValueWithMeta::String(format!("{}{}", a, b), meta.clone()),
            _ => bail!("unsupported types for + operator at {:?}", meta.0),
        },
        BinaryOperator::Sub => match (left_val.clone(), right_val.clone()) {
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => BamlValueWithMeta::Int(a - b, meta.clone()),
            (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Float(b,_)) => BamlValueWithMeta::Float(a - b, meta.clone()),
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Float(b,_)) => BamlValueWithMeta::Float((a as f64) - b, meta.clone()),
            (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Int(b,_)) => BamlValueWithMeta::Float(a - (b as f64), meta.clone()),
            _ => bail!("unsupported types for - operator at {:?}", meta.0),
        },
        BinaryOperator::Mul => match (left_val.clone(), right_val.clone()) {
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => BamlValueWithMeta::Int(a * b, meta.clone()),
            (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Float(b,_)) => BamlValueWithMeta::Float(a * b, meta.clone()),
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Float(b,_)) => BamlValueWithMeta::Float((a as f64) * b, meta.clone()),
            (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Int(b,_)) => BamlValueWithMeta::Float(a * (b as f64), meta.clone()),
            _ => bail!("unsupported types for * operator at {:?}", meta.0),
        },
        BinaryOperator::Div => match (left_val.clone(), right_val.clone()) {
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => {
                if b == 0 { bail!("division by zero at {:?}", meta.0); }
                BamlValueWithMeta::Float((a as f64) / (b as f64), meta.clone())
            },
            (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Float(b,_)) => {
                if b == 0.0 { bail!("division by zero at {:?}", meta.0); }
                BamlValueWithMeta::Float(a / b, meta.clone())
            },
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Float(b,_)) => {
                if b == 0.0 { bail!("division by zero at {:?}", meta.0); }
                BamlValueWithMeta::Float((a as f64) / b, meta.clone())
            },
            (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Int(b,_)) => {
                if b == 0 { bail!("division by zero at {:?}", meta.0); }
                BamlValueWithMeta::Float(a / (b as f64), meta.clone())
            },
            _ => bail!("unsupported types for / operator at {:?}", meta.0),
        },
        BinaryOperator::Mod => match (left_val.clone(), right_val.clone()) {
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => {
                if b == 0 { bail!("modulo by zero at {:?}", meta.0); }
                BamlValueWithMeta::Int(a % b, meta.clone())
            },
            _ => bail!("unsupported types for % operator at {:?}", meta.0),
        },
        
        // Comparison operations
        BinaryOperator::Eq => {
            let equal = values_equal(&left_val.clone(), &right_val.clone());
            BamlValueWithMeta::Bool(equal, meta.clone())
        },
        BinaryOperator::Neq => {
            let not_equal = !values_equal(&left_val.clone(), &right_val.clone());
            BamlValueWithMeta::Bool(not_equal, meta.clone())
        },
        BinaryOperator::Lt => {
            let ord_opt = compare_values(&left_val.clone(), &right_val.clone())?;
            let less = ord_opt.map(|ord| matches!(ord, std::cmp::Ordering::Less))
                .ok_or_else(|| anyhow!("unsupported types for < operator at {:?}", meta.0))?;
            BamlValueWithMeta::Bool(less, meta.clone())
        },
        BinaryOperator::LtEq => {
            let ord_opt = compare_values(&left_val.clone(), &right_val.clone())?;
            let less_eq = ord_opt.map(|ord| matches!(ord, std::cmp::Ordering::Less | std::cmp::Ordering::Equal))
                .ok_or_else(|| anyhow!("unsupported types for <= operator at {:?}", meta.0))?;
            BamlValueWithMeta::Bool(less_eq, meta.clone())
        },
        BinaryOperator::Gt => {
            let ord_opt = compare_values(&left_val.clone(), &right_val.clone())?;
            let greater = ord_opt.map(|ord| matches!(ord, std::cmp::Ordering::Greater))
                .ok_or_else(|| anyhow!("unsupported types for > operator at {:?}", meta.0))?;
            BamlValueWithMeta::Bool(greater, meta.clone())
        },
        BinaryOperator::GtEq => {
            let ord_opt = compare_values(&left_val.clone(), &right_val.clone())?;
            let greater_eq = ord_opt.map(|ord| matches!(ord, std::cmp::Ordering::Greater | std::cmp::Ordering::Equal))
                .ok_or_else(|| anyhow!("unsupported types for >= operator at {:?}", meta.0))?;
            BamlValueWithMeta::Bool(greater_eq, meta.clone())
        },
        
        // Logical operations
        BinaryOperator::And => {
            match left_val.clone() {
                BamlValueWithMeta::Bool(false,_) => BamlValueWithMeta::Bool(false, meta.clone()),
                BamlValueWithMeta::Bool(true,_) => {
                    match right_val.clone() {
                        BamlValueWithMeta::Bool(b,_) => BamlValueWithMeta::Bool(b, meta.clone()),
                        _ => bail!("right operand of && must be bool at {:?}", meta.0),
                    }
                },
                _ => bail!("left operand of && must be bool at {:?}", meta.0),
            }
        },
        BinaryOperator::Or => {
            match left_val.clone() {
                BamlValueWithMeta::Bool(true,_) => BamlValueWithMeta::Bool(true, meta.clone()),
                BamlValueWithMeta::Bool(false,_) => {
                    match right_val.clone() {
                        BamlValueWithMeta::Bool(b,_) => BamlValueWithMeta::Bool(b, meta.clone()),
                        _ => bail!("right operand of || must be bool at {:?}", meta.0),
                    }
                },
                _ => bail!("left operand of || must be bool at {:?}", meta.0),
            }
        },
        
        // Bitwise operations (integer only)
        BinaryOperator::BitAnd => match (left_val.clone(), right_val.clone()) {
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => BamlValueWithMeta::Int(a & b, meta.clone()),
            _ => bail!("bitwise & requires integer operands at {:?}", meta.0),
        },
        BinaryOperator::BitOr => match (left_val.clone(), right_val.clone()) {
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => BamlValueWithMeta::Int(a | b, meta.clone()),
            _ => bail!("bitwise | requires integer operands at {:?}", meta.0),
        },
        BinaryOperator::BitXor => match (left_val.clone(), right_val.clone()) {
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => BamlValueWithMeta::Int(a ^ b, meta.clone()),
            _ => bail!("bitwise ^ requires integer operands at {:?}", meta.0),
        },
        BinaryOperator::Shl => match (left_val.clone(), right_val.clone()) {
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => {
                if b < 0 { bail!("negative shift amount at {:?}", meta.0); }
                BamlValueWithMeta::Int(a << b, meta.clone())
            },
            _ => bail!("shift << requires integer operands at {:?}", meta.0),
        },
        BinaryOperator::Shr => match (left_val.clone(), right_val.clone()) {
            (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => {
                if b < 0 { bail!("negative shift amount at {:?}", meta.0); }
                BamlValueWithMeta::Int(a >> b, meta.clone())
            },
            _ => bail!("shift >> requires integer operands at {:?}", meta.0),
        },
    })
}

fn evaluate_unary_op(
    operator: &crate::hir::UnaryOperator,
    val: &BamlValueWithMeta<ExprMetadata>,
    meta: &ExprMetadata,
) -> Result<BamlValueWithMeta<ExprMetadata>> {
    use crate::hir::UnaryOperator;
    Ok(match operator {
        UnaryOperator::Not => match val.clone() {
            BamlValueWithMeta::Bool(b,_) => BamlValueWithMeta::Bool(!b, meta.clone()),
            _ => bail!("! operator requires boolean operand at {:?}", meta.0),
        },
        UnaryOperator::Neg => match val.clone() {
            BamlValueWithMeta::Int(i,_) => BamlValueWithMeta::Int(-i, meta.clone()),
            BamlValueWithMeta::Float(f,_) => BamlValueWithMeta::Float(-f, meta.clone()),
            _ => bail!("- operator requires numeric operand at {:?}", meta.0),
        },
    })
}

fn values_equal(left: &BamlValueWithMeta<ExprMetadata>, right: &BamlValueWithMeta<ExprMetadata>) -> bool {
    match (left, right) {
        (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => a == b,
        (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Float(b,_)) => a == b,
        (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Float(b,_)) => *a as f64 == *b,
        (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Int(b,_)) => *a == *b as f64,
        (BamlValueWithMeta::String(a, _), BamlValueWithMeta::String(b, _)) => a == b,
        (BamlValueWithMeta::Null(_), BamlValueWithMeta::Null(_)) => true,
        _ => false,
    }
}

fn compare_values(left: &BamlValueWithMeta<ExprMetadata>, right: &BamlValueWithMeta<ExprMetadata>) -> Result<Option<std::cmp::Ordering>> {
    Ok(match (left, right) {
        (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Int(b,_)) => Some(a.cmp(b)),
        (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Float(b,_)) => a.partial_cmp(b),
        (BamlValueWithMeta::Int(a,_), BamlValueWithMeta::Float(b,_)) => (*a as f64).partial_cmp(b),
        (BamlValueWithMeta::Float(a,_), BamlValueWithMeta::Int(b,_)) => a.partial_cmp(&(*b as f64)),
        (BamlValueWithMeta::String(a, _), BamlValueWithMeta::String(b, _)) => Some(a.cmp(b)),
        _ => None,
    })
}

// /// Convert a BamlValue to BamlValueWithMeta by adding the given metadata
// fn baml_value_to_with_meta(value: &BamlValue, meta: ExprMetadata) -> BamlValueWithMeta<ExprMetadata> {
//     match value {
//         BamlValue::String(s) => BamlValueWithMeta::String(s.clone(), meta),
//         BamlValue::Int(i) => BamlValueWithMeta::Int(*i, meta),
//         BamlValue::Float(f) => BamlValueWithMeta::Float(*f, meta),
//         BamlValue::Bool(b) => BamlValueWithMeta::Bool(*b, meta),
//         BamlValue::Map(m) => {
//             let with_meta_map = m.iter()
//                 .map(|(k, v)| (k.clone(), baml_value_to_with_meta(v, meta.clone())))
//                 .collect();
//             BamlValueWithMeta::Map(with_meta_map, meta)
//         },
//         BamlValue::List(l) => {
//             let with_meta_list = l.iter()
//                 .map(|v| baml_value_to_with_meta(v, meta.clone()))
//                 .collect();
//             BamlValueWithMeta::List(with_meta_list, meta)
//         },
//         BamlValue::Media(m) => BamlValueWithMeta::Media(m.clone(), meta),
//         BamlValue::Enum(name, val) => BamlValueWithMeta::Enum(name.clone(), val.clone(), meta),
//         BamlValue::Class(name, fields) => {
//             let with_meta_fields = fields.iter()
//                 .map(|(k, v)| (k.clone(), baml_value_to_with_meta(v, meta.clone())))
//                 .collect();
//             BamlValueWithMeta::Class(name.clone(), with_meta_fields, meta)
//         },
//         BamlValue::Null => BamlValueWithMeta::Null(meta),
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thir::THir;
    use internal_baml_diagnostics::Span;

    fn meta() -> ExprMetadata {
        (Span::fake(), None)
    }

    fn empty_thir() -> THir<ExprMetadata> {
        THir {
            expr_functions: vec![],
            llm_functions: vec![],
            global_assignments: BamlMap::new(),
            classes: BamlMap::new(),
            enums: BamlMap::new(),
        }
    }

    #[test]
    fn eval_atom_int() {
        let thir = empty_thir();
        let expr = Expr::Atom(BamlValueWithMeta::Int(1, meta()));
        let out = super::interpret_thir(thir, expr).unwrap();
        match out {
            BamlValueWithMeta::Int(i, _) => assert_eq!(i, 1),
            v => panic!("expected int, got {:?}", v),
        }
    }

    #[test]
    fn eval_function_call_identity() {
        let thir = empty_thir();
        let body = Block {
            env: BamlMap::new(),
            statements: vec![],
            return_value: Expr::BoundVar(VarIndex { de_bruijn: 0, tuple: 0 }, meta()),
            span: Span::fake(),
        };

        let func = Expr::Function(1, Arc::new(body), meta());
        let call = Expr::Call {
            func: Arc::new(func),
            type_args: vec![],
            args: vec![Expr::Atom(BamlValueWithMeta::Int(42, meta()))],
            meta: meta(),
        };

        let out = super::interpret_thir(thir, call).unwrap();
        match out {
            BamlValueWithMeta::Int(i, _) => assert_eq!(i, 42),
            v => panic!("expected int, got {:?}", v),
        }
    }

    #[test]
    fn eval_function_uses_global() {
        let mut thir = empty_thir();
        thir.global_assignments.insert(
            "x".to_string(),
            Expr::Atom(BamlValueWithMeta::Int(7, meta())),
        );

        // Function with arity 0 returning free var `x`
        let body = Block {
            env: BamlMap::new(),
            statements: vec![],
            return_value: Expr::FreeVar("x".to_string(), meta()),
            span: Span::fake(),
        };
        let func = Expr::Function(0, Arc::new(body), meta());
        let call = Expr::Call { func: Arc::new(func), type_args: vec![], args: vec![], meta: meta() };

        let out = super::interpret_thir(thir, call).unwrap();
        match out {
            BamlValueWithMeta::Int(i, _) => assert_eq!(i, 7),
            v => panic!("expected int, got {:?}", v),
        }
    }
}