#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{parse_macro_input, parse_quote, parse_quote_spanned, spanned::Spanned, Attribute, Block, Error, Expr, ItemFn, ExprMacro, Result, Stmt};
use quote::ToTokens;
///Print all nested logging events to the console.
///
///## Usage
///
///The [`log`](macro@log) attribute macro is used to print all logging-related
///events in the function it is applied to. This includes all calls
///made in nested functions. If [`log`](macro@log) is applied to a nested function,
///two separate reports will be generated.
///
///```
///use report::{log, info};
///
///#[log("First report")]
///fn main() {
///    info!("This info is attached to the first report");
///    function();
///    nested()
///}
///
///#[log("Second report")]
///fn function() {
///    info!("This info is attached to the second report");
///}
///
///
///fn nested() {
///    info!("This info is also attached to the first report");
///}
///```
///
///```text
///╭────────────────────────────────────────────────────────────────────────────────────────────╮
///│ Second report                                                                              │
///├─┬──────────────────────────────────────────────────────────────────────────────────────────┤
///│ ╰── info: This info is attached to the second report                                       │
///╰────────────────────────────────────────────────────────────────────────────────────────────╯
///╭────────────────────────────────────────────────────────────────────────────────────────────╮
///│ First report                                                                               │
///├─┬──────────────────────────────────────────────────────────────────────────────────────────┤
///│ ├── info: This info is attached to the first report                                        │
///│ ╰── info: This info is also attached to the first report                                   │
///╰────────────────────────────────────────────────────────────────────────────────────────────╯
///```
///
///## Formatting arguments
///
///It's possible to use the arguments of the function in the format
///string.
///
///```
///use report::log;
///
///#[log("The argument is {}", arg)]
///fn function(arg: i32) -> i32 {
///    return arg;
///}
///```
///
///This macro should only be used in application code and not in
///libraries, so that a user can integrate generated reports into
///their own, making the grouping of related information easier.
#[proc_macro_attribute]
pub fn log(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as ItemFn);
    let args = TokenStream2::from(args);

    item.block.stmts.insert(0, parse_quote!(
        let _logger = ::report::Report::log(|| format!(#args));
    ));

    return TokenStream::from(item.to_token_stream())
}

///Annotate a new logging group with a custom message.
///
///## Usage
///
///Currently, the ability to use proc macro attributes on expressions
///is only available when enabling the `proc-macro-hygiene` feature
///using the nightly compiler. To circumvent this limitation, [`report`](macro@report)
///will parse the functions and expand all subsequent calls to the
///similarly named attribute macro. In the following steps, any expression
///annotated with a report will display all nested logging events under
///the same group. If there are no events to be logged, the group header
///will not be included in the final report.
///
///```
///use report::{report, info, log};
///
///#[report]
///#[log("Test report")]
///fn main() {
///    
///    #[report("First group")]
///    {
///        info!("This info is attached to the first group");
///    }
///
///    #[report("Second group")]
///    info!("This info is attached to the second group");
///
///    #[report("Omitted group")]
///    {
///        //This group will not be included, since there are no events
///    }
///}
///```
///
///```text
///╭────────────────────────────────────────────────────────────────────────────────────────────╮
///│ Test report                                                                                │
///├─┬──────────────────────────────────────────────────────────────────────────────────────────┤
///│ ├── First group                                                                            │
///│ │   ╰── info: This info is attached to the first group                                     │
///│ ╰── Second group                                                                           │
///│     ╰── info: This info is attached to the second group                                    │
///╰────────────────────────────────────────────────────────────────────────────────────────────╯
///```
///
///## Borrowing of format arguments
///
///Just like any other macro in this crate, the format string used by
///the println! macro can be supplied as an argument. It is, however,
///important to note that the actual string formatting happens **after**
///the expression is evaluated. This means all arguments will be borrowed
///for the scope of the expression and will make it impossible to use
///elements that are moved and do not implement the `Copy`
///trait. Mutable references can’t be used in the formatting string, either.
///The reason for this is that a group may be excluded from the report,
///making it a waste of resources to format the string in advance.
///
///```compile_fail
///use report::{Result, report};
///use std::fs::File;
///
///#[report]
///fn open_file() -> Result {
///    let path = String::from("Cargo.toml");
///    #[report("Opening file {path:?}")]
///    let _file = File::open(path)?; //cannot move out of 'path' because it is borrowed
///    Ok(())
///}
///```
///
///```
///use report::{Result, report};
///use std::fs::File;
///
///#[report]
///fn open_file() -> Result {
///    let path = String::from("Cargo.toml");
///    #[report("Opening file {path:?}")]
///    let _file = File::open(path.as_str())?; //this works because path is no longer moved
///    Ok(())
///}
///```

#[proc_macro_attribute]
pub fn report(args: TokenStream, input: TokenStream) -> TokenStream {

    let mut item = parse_macro_input!(input as ItemFn);

    if !args.is_empty() {
        let args = TokenStream2::from(args);
        let mut error = Error::new_spanned(args, "This attribute does not take any arguments")
            .to_compile_error();
        error.extend(item.to_token_stream());
        return TokenStream::from(error);    
    }

    if let Err(err) = iter_block(&mut item.block) {
        return TokenStream::from(err.to_compile_error())
    }

    return TokenStream::from(item.to_token_stream())
}

fn process_expr(expr: &mut Expr, local_attrs: Option<&mut Vec<Attribute>>) -> Result<()> {
    iter_expr(expr)?;

    let mut attrs = Vec::new();

    if let Some(local_attrs) = local_attrs {
        local_attrs.retain(|attr| {
            let res = !attr.path().is_ident("report");
            if !res { attrs.push(attr.clone()) }
            res
        });
    }

    if let Some(expr_attrs) = get_attrs(expr) {
        expr_attrs.retain(|attr| {
            let res = !attr.path().is_ident("report");
            if !res { attrs.push(attr.clone()) }
            res
        });
    }

    for attr in attrs {
        let list = attr.meta.require_list()?.tokens.clone(); 
        *expr = parse_quote_spanned!(attr.span() => {
            let _logger = ::report::Report::rec(|| format!(#list));
            #expr
        });
    }

    return Ok(());
}

fn iter_block(block: &mut Block) -> Result<()> {
    for statement in block.stmts.iter_mut() {
        match statement {
            Stmt::Local(local) => if let Some(init) = local.init.as_mut() {
                process_expr(&mut init.expr, Some(&mut local.attrs))?;
                if let Some((.., expr)) = init.diverge.as_mut() {
                    iter_expr(expr)?;
                }
            },
            Stmt::Expr(expr, ..) => {
                process_expr(expr, None)?;
            },
            Stmt::Macro(macro_expr) => {
                let mut attrs = Vec::new();

                macro_expr.attrs.retain(|attr| {
                    let res = !attr.path().is_ident("report");
                    if !res { attrs.push(attr.clone()) }
                    res
                });

                if attrs.is_empty() { continue } 
                let mut expr = Expr::Macro(ExprMacro {
                    attrs,
                    mac: macro_expr.mac.clone()
                });

                process_expr(&mut expr, None)?;
                *statement = Stmt::Expr(expr, macro_expr.semi_token)
            },
            Stmt::Item(..) => ()
        }
    }
    Ok(())
}

fn iter_expr(expr: &mut Expr) -> Result<()> {
    match expr {
        Expr::Try(try_expr) => process_expr(&mut try_expr.expr, None),
        Expr::Call(call_expr) => {
            process_expr(&mut call_expr.func, None)?;
            for arg in call_expr.args.iter_mut() {
                process_expr(arg, None)?;
            }
            Ok(())
        },
        Expr::Tuple(tuple_expr) => {
            for expr in tuple_expr.elems.iter_mut() {
                process_expr(expr, None)?;
            }
            Ok(())
        },
        Expr::Macro(..) => Ok(()),
        Expr::MethodCall(method_call_expr) => {
            process_expr(&mut method_call_expr.receiver, None)?;
            for arg in method_call_expr.args.iter_mut() {
                process_expr(arg, None)?;
            }
            Ok(())
        },
        Expr::Match(match_expr) => {
            process_expr(&mut match_expr.expr, None)?;
            for arm in match_expr.arms.iter_mut() {
                process_expr(arm.body.as_mut(), Some(arm.attrs.as_mut()))?;
            }
            Ok(())
        },
        Expr::Closure(closure_expr) => {
            process_expr(&mut closure_expr.body, None)?;
            Ok(())
        },
        Expr::Unsafe(unsafe_expr) => iter_block(&mut unsafe_expr.block),
        Expr::Block(block_expr) => iter_block(&mut block_expr.block),
        Expr::Assign(assign_expr) => {
            process_expr(&mut assign_expr.left, None)?;
            process_expr(&mut assign_expr.right, None)
        },
        Expr::Field(field_expr) => process_expr(&mut field_expr.base, None),
        Expr::Index(index_expr) => {
            process_expr(&mut index_expr.expr, None)?;
            process_expr(&mut index_expr.index, None)
        },
        Expr::Range(range_expr) => {
            if let Some(start) = range_expr.start.as_mut() {
                process_expr(start, None)?;
            }
            if let Some(end) = range_expr.end.as_mut() {
                process_expr(end, None)?;
            }
            Ok(())
        },
        Expr::Path(..) => Ok(()),
        Expr::Reference(reference_expr) => process_expr(&mut reference_expr.expr, None),
        Expr::Break(break_expr) => if let Some(expr) = break_expr.expr.as_mut() {
            process_expr(expr, None)
        } else { Ok(()) },
        Expr::Continue(..) => Ok(()),
        Expr::Return(return_expr) => if let Some(expr) = return_expr.expr.as_mut() {
            process_expr(expr, None)
        } else { Ok(()) },
        Expr::Struct(struct_expr) => {
            for field in struct_expr.fields.iter_mut() {
                process_expr(&mut field.expr, None)?;
            }
            Ok(())
        },
        Expr::Repeat(repeat_expr) => {
            process_expr(&mut repeat_expr.expr, None)?;
            process_expr(&mut repeat_expr.len, None)
        },
        Expr::Paren(paren_expr) => process_expr(&mut paren_expr.expr, None),
        Expr::Group(group_expr) => process_expr(&mut group_expr.expr, None),
        Expr::TryBlock(try_block_expr) => iter_block(&mut try_block_expr.block),
        Expr::Async(async_expr) => iter_block(&mut async_expr.block),
        Expr::Await(await_expr) => process_expr(&mut await_expr.base, None),
        Expr::Yield(yield_expr) => if let Some(expr) = yield_expr.expr.as_mut() {
            process_expr(expr, None)
        } else { Ok(()) },
        Expr::ForLoop(for_loop_expr) => {
            process_expr(&mut for_loop_expr.expr, None)?;
            iter_block(&mut for_loop_expr.body)
        },
        Expr::While(while_expr) => {
            process_expr(&mut while_expr.cond, None)?;
            iter_block(&mut while_expr.body)
        },
        Expr::Loop(loop_expr) => iter_block(&mut loop_expr.body),
        Expr::If(if_expr) => {
            process_expr(&mut if_expr.cond, None)?;
            iter_block(&mut if_expr.then_branch)?;
            if let Some((_, else_branch)) = if_expr.else_branch.as_mut() {
                process_expr(else_branch, None)?;
            }
            Ok(())
        },
        Expr::Let(let_expr) => process_expr(&mut let_expr.expr, None),
        Expr::Lit(..) => Ok(()),
        Expr::Cast(cast_expr) => {
            process_expr(&mut cast_expr.expr, None)?;
            Ok(())
        },
        Expr::Infer(..) => Ok(()),
        Expr::Array(array_expr) => {
            for expr in array_expr.elems.iter_mut() {
                process_expr(expr, None)?;
            }
            Ok(())
        },
        Expr::Unary(unary_expr) => process_expr(&mut unary_expr.expr, None),
        Expr::Binary(binary_expr) => {
            process_expr(&mut binary_expr.left, None)?;
            process_expr(&mut binary_expr.right, None)
        },
        Expr::Const(const_expr) => iter_block(&mut const_expr.block),
        _ => Ok(())
    }
}

fn get_attrs(expr: &mut Expr) -> Option<&mut Vec<Attribute>> {
    match expr {
        Expr::Try(try_expr) => Some(&mut try_expr.attrs),
        Expr::Call(call_expr) => Some(&mut call_expr.attrs),
        Expr::Tuple(tuple_expr) => Some(&mut tuple_expr.attrs),
        Expr::Macro(macro_expr) => Some(&mut macro_expr.attrs),
        Expr::MethodCall(method_call_expr) => Some(&mut method_call_expr.attrs),
        Expr::Match(match_expr) => Some(&mut match_expr.attrs),
        Expr::Closure(closure_expr) => Some(&mut closure_expr.attrs),
        Expr::Unsafe(unsafe_expr) => Some(&mut unsafe_expr.attrs),
        Expr::Block(block_expr) => Some(&mut block_expr.attrs),
        Expr::Assign(assign_expr) => Some(&mut assign_expr.attrs),
        Expr::Field(field_expr) => Some(&mut field_expr.attrs),
        Expr::Index(index_expr) => Some(&mut index_expr.attrs),
        Expr::Range(range_expr) => Some(&mut range_expr.attrs),
        Expr::Path(path_expr) => Some(&mut path_expr.attrs),
        Expr::Reference(reference_expr) => Some(&mut reference_expr.attrs),
        Expr::Break(break_expr) => Some(&mut break_expr.attrs),
        Expr::Continue(continue_expr) => Some(&mut continue_expr.attrs),
        Expr::Return(return_expr) => Some(&mut return_expr.attrs),
        Expr::Struct(struct_expr) => Some(&mut struct_expr.attrs),
        Expr::Repeat(repeat_expr) => Some(&mut repeat_expr.attrs),
        Expr::Paren(paren_expr) => Some(&mut paren_expr.attrs),
        Expr::Group(group_expr) => Some(&mut group_expr.attrs),
        Expr::TryBlock(try_block_expr) => Some(&mut try_block_expr.attrs),
        Expr::Async(async_expr) => Some(&mut async_expr.attrs),
        Expr::Await(await_expr) => Some(&mut await_expr.attrs),
        Expr::Yield(yield_expr) => Some(&mut yield_expr.attrs),
        Expr::ForLoop(for_loop_expr) => Some(&mut for_loop_expr.attrs),
        Expr::While(while_expr) => Some(&mut while_expr.attrs),
        Expr::Loop(loop_expr) => Some(&mut loop_expr.attrs),
        Expr::If(if_expr) => Some(&mut if_expr.attrs),
        Expr::Let(let_expr) => Some(&mut let_expr.attrs),
        Expr::Lit(lit_expr) => Some(&mut lit_expr.attrs),
        Expr::Cast(cast_expr) => Some(&mut cast_expr.attrs),
        Expr::Infer(info_expr) => Some(&mut info_expr.attrs),
        Expr::Array(array_expr) => Some(&mut array_expr.attrs),
        Expr::Unary(unary_expr) => Some(&mut unary_expr.attrs),
        Expr::Binary(binary_expr) => Some(&mut binary_expr.attrs),
        Expr::Const(const_expr) => Some(&mut const_expr.attrs),
        _ => None
    }
}
