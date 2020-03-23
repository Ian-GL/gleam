#[cfg(test)]
mod tests;

use crate::ast::*;
use crate::pretty::*;
use itertools::Itertools;

const INDENT: isize = 2;

pub fn pretty(src: &str) -> Result<String, crate::parser::LalrpopError> {
    let stripped = crate::parser::strip_extra(src.as_ref());
    let ast = crate::grammar::ModuleParser::new()
        .parse(&stripped)
        .map_err(|e| e.map_token(|crate::grammar::Token(a, b)| (a, b.to_string())))?;
    Ok(pretty_module(&ast))
}

pub fn pretty_module(m: &UntypedModule) -> String {
    format(80, module(m))
}

fn module(module: &UntypedModule) -> Document {
    let mut has_imports = false;
    let mut has_other = false;

    let imports = concat(
        module
            .statements
            .iter()
            .filter_map(|s| match s {
                import @ Statement::Import { .. } => {
                    has_imports = true;
                    Some(import.to_doc())
                }
                _ => None,
            })
            .intersperse(line()),
    );

    let statements = concat(
        module
            .statements
            .iter()
            .filter_map(|s| match s {
                Statement::Import { .. } => None,
                statement => {
                    has_other = true;
                    Some(statement.to_doc())
                }
            })
            .intersperse(lines(2)),
    );

    let sep = if has_imports && has_other {
        lines(2)
    } else {
        nil()
    };

    imports.append(sep).append(statements).append(line())
}

impl Documentable for &ArgNames {
    fn to_doc(self) -> Document {
        match self {
            ArgNames::Discard => "_".to_string(),
            ArgNames::LabelledDiscard { label } => format!("{} _", label),
            ArgNames::Named { name } => name.to_string(),
            ArgNames::NamedLabelled { name, label } => format!("{} {}", label, name),
        }
        .to_doc()
    }
}

impl Documentable for &Arg {
    fn to_doc(self) -> Document {
        self.names
            .to_doc()
            .append(match &self.annotation {
                Some(a) => ": ".to_doc().append(a),
                None => nil(),
            })
            .group()
    }
}

impl Documentable for &RecordConstructor {
    fn to_doc(self) -> Document {
        if self.args.is_empty() {
            self.name.clone().to_doc()
        } else {
            self.name
                .to_string()
                .to_doc()
                .append(wrap_args(self.args.iter().map(
                    |(label, typ)| match label {
                        Some(l) => l.to_string().to_doc().append(": ").append(typ),
                        None => typ.to_doc(),
                    },
                )))
        }
    }
}

impl Documentable for &UntypedStatement {
    fn to_doc(self) -> Document {
        match self {
            Statement::Fn {
                name,
                args,
                body,
                public,
                return_annotation,
                ..
            } => pub_(public)
                .append(format!("fn {}", name))
                .append(wrap_args(args.iter().map(|e| e.to_doc())))
                .append(if let Some(anno) = return_annotation {
                    " -> ".to_doc().append(anno)
                } else {
                    nil()
                })
                .append(" {")
                .append(line().append(body).nest(INDENT).group())
                .append(line())
                .append("}"),

            Statement::TypeAlias {
                alias,
                args,
                resolved_type,
                public,
                ..
            } => pub_(public)
                .append("type ")
                .append(alias.to_string())
                .append(if args.is_empty() {
                    nil()
                } else {
                    wrap_args(args.iter().map(|e| e.clone().to_doc()))
                })
                .append(" =")
                .append(line().append(resolved_type).group().nest(INDENT)),

            Statement::CustomType {
                name,
                args,
                public,
                constructors,
                ..
            } => pub_(public)
                .to_doc()
                .append("type ")
                .append(if args.is_empty() {
                    name.clone().to_doc()
                } else {
                    name.to_string()
                        .to_doc()
                        .append(wrap_args(args.iter().map(|e| e.clone().to_doc())))
                })
                .append(" {")
                .append(concat(
                    constructors
                        .into_iter()
                        .map(|c| line().append(c).nest(INDENT).group()),
                ))
                .append(line())
                .append("}"),

            Statement::ExternalFn {
                public,
                args,
                name,
                retrn,
                module,
                fun,
                ..
            } => pub_(public)
                .to_doc()
                .append("external fn ")
                .group()
                .append(name.to_string())
                .append(wrap_args(args.iter().map(|e| e.to_doc())))
                .append(" -> ".to_doc())
                .append(retrn)
                .append(" =")
                .append(line())
                .append(format!("  \"{}\" ", module))
                .append(format!("\"{}\"", fun)),

            Statement::ExternalType {
                public, name, args, ..
            } => pub_(public)
                .append("external type ")
                .append(name.to_string())
                .append(if args.is_empty() {
                    nil()
                } else {
                    wrap_args(args.iter().map(|e| e.clone().to_doc()))
                }),

            Statement::Import {
                module,
                as_name,
                unqualified,
                ..
            } => nil()
                .append("import ")
                .append(module.join("/"))
                .append(if unqualified.is_empty() {
                    nil()
                } else {
                    ".{".to_doc()
                        .append(concat(
                            unqualified
                                .iter()
                                .map(|e| e.clone().to_doc())
                                .intersperse(", ".to_doc()),
                        ))
                        .append("}")
                })
                .append(if let Some(name) = as_name {
                    format!(" as {}", name).to_doc()
                } else {
                    nil()
                }),
        }
    }
}

fn pub_(public: &bool) -> Document {
    if *public {
        "pub ".to_doc()
    } else {
        nil()
    }
}

impl Documentable for &UnqualifiedImport {
    fn to_doc(self) -> Document {
        self.name.clone().to_doc().append(match &self.as_name {
            None => nil(),
            Some(s) => " as ".to_doc().append(s.clone()),
        })
    }
}

impl Documentable for &ExternalFnArg {
    fn to_doc(self) -> Document {
        label(&self.label).append(self.typ.to_doc())
    }
}

fn label(label: &Option<String>) -> Document {
    match label {
        Some(s) => s.clone().to_doc().append(": "),
        None => nil(),
    }
}

impl Documentable for &CallArg<UntypedExpr> {
    fn to_doc(self) -> Document {
        match &self.label {
            Some(s) => s.clone().to_doc().append(": "),
            None => nil(),
        }
        .append(&self.value)
    }
}

impl Documentable for &CallArg<UntypedPattern> {
    fn to_doc(self) -> Document {
        match &self.label {
            Some(s) => s.clone().to_doc().append(": "),
            None => nil(),
        }
        .append(&self.value)
    }
}

impl Documentable for &BinOp {
    fn to_doc(self) -> Document {
        match self {
            BinOp::And => " && ",
            BinOp::Or => " || ",
            BinOp::LtInt => " < ",
            BinOp::LtEqInt => " <= ",
            BinOp::LtFloat => " <. ",
            BinOp::LtEqFloat => " <=. ",
            BinOp::Eq => " == ",
            BinOp::NotEq => " != ",
            BinOp::GtEqInt => " >= ",
            BinOp::GtInt => " > ",
            BinOp::GtEqFloat => " >=. ",
            BinOp::GtFloat => " >. ",
            BinOp::AddInt => " + ",
            BinOp::AddFloat => " +. ",
            BinOp::SubInt => " - ",
            BinOp::SubFloat => " -. ",
            BinOp::MultInt => " * ",
            BinOp::MultFloat => " *. ",
            BinOp::DivInt => " / ",
            BinOp::DivFloat => " /. ",
            BinOp::ModuloInt => " % ",
        }
        .to_doc()
    }
}

impl Documentable for &UntypedPattern {
    fn to_doc(self) -> Document {
        match self {
            Pattern::Int { value, .. } => value.to_doc(),

            Pattern::Float { value, .. } => value.to_doc(),

            Pattern::String { value, .. } => value.clone().to_doc().surround("\"", "\""),

            Pattern::Var { name, .. } => name.to_string().to_doc(),

            Pattern::Let { name, pattern, .. } => {
                pattern.to_doc().append(" as ").append(name.to_string())
            }

            Pattern::Discard { .. } => "_".to_doc(),

            Pattern::Nil { .. } => "[]".to_doc(),

            Pattern::Cons { head, tail, .. } => head
                .to_doc()
                .append("|")
                .append(tail.as_ref())
                .surround("[", "]"),

            Pattern::Constructor {
                name,
                args,
                module: None,
                ..
            } if args.is_empty() => name.to_string().to_doc(),

            Pattern::Constructor {
                name,
                args,
                module: Some(m),
                ..
            } if args.is_empty() => m.to_string().to_doc().append(".").append(name.to_string()),

            Pattern::Constructor {
                name,
                args,
                module: None,
                ..
            } => name
                .to_string()
                .to_doc()
                .append(wrap_args(args.iter().map(|a| a.to_doc()))),

            Pattern::Constructor {
                name,
                args,
                module: Some(m),
                ..
            } => m
                .to_string()
                .to_doc()
                .append(".")
                .append(name.to_string())
                .append(wrap_args(args.iter().map(|a| a.to_doc()))),

            Pattern::Tuple { elems, .. } => "tuple"
                .to_doc()
                .append(wrap_args(elems.iter().map(|e| e.to_doc()))),
        }
    }
}

impl Documentable for &UntypedClause {
    fn to_doc(self) -> Document {
        "pattern"
            .to_doc()
            .append(clause_guard(&self.guard))
            .append(" -> ")
            .append(&self.then)
            .append(line());
        todo!()
    }
}

fn clause_guard(_guard: &Option<UntypedClauseGuard>) -> Document {
    todo!()
}

impl Documentable for &UntypedExpr {
    fn to_doc(self) -> Document {
        match self {
            UntypedExpr::Todo { .. } => "todo".to_doc(),

            UntypedExpr::Pipe { left, right, .. } => left
                .to_doc()
                .append(force_break())
                .append(line())
                .append("|> ")
                .append(right.as_ref()),

            UntypedExpr::Int { value, .. } => value.to_doc(),

            UntypedExpr::Float { value, .. } => value.to_doc(),

            UntypedExpr::String { value, .. } => value.clone().to_doc().surround("\"", "\""),

            UntypedExpr::Seq { first, then, .. } => first
                .to_doc()
                .append(force_break())
                .append(line())
                .append(then.as_ref()),

            UntypedExpr::Var { name, .. } => name.clone().to_doc(),

            UntypedExpr::TupleIndex { tuple, index, .. } => {
                tuple.to_doc().append(".").append(*index)
            }

            UntypedExpr::Fn {
                // is_capture, // TODO: render captures
                // return_annotation, // TODO: render this annotation
                args,
                body,
                ..
            } => "fn("
                .to_doc()
                .append(wrap_args(args.iter().map(|e| e.to_doc())).nest_current())
                .append(")")
                .append(" {\n")
                .append(body.as_ref())
                .append("\n}"),

            UntypedExpr::Nil { .. } => "[]".to_doc(),

            UntypedExpr::Cons { head, tail, .. } => {
                Document::Cons(Box::new(head.to_doc().append("|")), Box::new(tail.to_doc()))
                    .surround("[", "]")
            }

            UntypedExpr::Call { fun, args, .. } => fun
                .to_doc()
                .append(wrap_args(args.iter().map(|e| e.to_doc()))),

            UntypedExpr::BinOp {
                name, left, right, ..
            } => left.to_doc().append(name).append(right.as_ref()),

            UntypedExpr::Let {
                value,
                pattern,
                then,
                ..
            } => "let "
                .to_doc()
                .append(pattern)
                .append(" = ")
                .append(value.as_ref())
                .append(line())
                .append(then.as_ref()),

            UntypedExpr::Case {
                subjects, clauses, ..
            } => "case "
                .to_doc()
                .append(concat(
                    subjects
                        .into_iter()
                        .map(|s| s.to_doc())
                        .intersperse(", ".to_doc()),
                ))
                .append(" {")
                .append(concat(
                    clauses
                        .into_iter()
                        .map(|c| line().append(c).nest(INDENT).group()),
                ))
                .append(line())
                .append("}"),

            UntypedExpr::FieldAccess {
                label, container, ..
            } => container.to_doc().append(format!(".{}", label)),

            UntypedExpr::Tuple { elems, .. } => "tuple"
                .to_doc()
                .append(wrap_args(elems.iter().map(|e| e.to_doc()))),
        }
    }
}

impl Documentable for &TypeAst {
    fn to_doc(self) -> Document {
        match self {
            TypeAst::Constructor { name, args, .. } if args.is_empty() => name.to_string().to_doc(),

            TypeAst::Constructor { name, args, .. } => name
                .to_string()
                .to_doc()
                .append(wrap_args(args.iter().map(|e| e.to_doc()))),

            TypeAst::Fn { args, retrn, .. } => "fn"
                .to_string()
                .to_doc()
                .append(wrap_args(args.iter().map(|e| e.to_doc())))
                .append(delim(" ->"))
                .append(retrn.to_doc()),

            TypeAst::Var { name, .. } => name.clone().to_doc(),

            TypeAst::Tuple { elems, .. } => "tuple"
                .to_doc()
                .append(wrap_args(elems.iter().map(|e| e.to_doc()))),
        }
    }
}

pub fn wrap_args<I>(args: I) -> Document
where
    I: Iterator<Item = Document>,
{
    let mut args = args.peekable();
    if let None = args.peek() {
        return "(".to_doc().append(break_("", "")).append(")").group();
    }
    break_("(", "(")
        .append(concat(args.intersperse(delim(","))))
        .nest(INDENT)
        .append(break_(",", ""))
        .append(")")
        .group()
}