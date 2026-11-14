//! Standard functions. Every function is pure and charged steps for the work it does.

use crate::{
    Value, date,
    eval::{ErrorKind, EvalError, EvalResult, Evaluator, compare, type_err},
    format,
    functions::FUNCTIONS,
    parser::Expr,
};

pub(crate) fn call(ev: &mut Evaluator<'_>, name: &str, args: &[Expr]) -> EvalResult {
    let a = Args { name, args };
    match name {
        "if" => {
            a.arity(3, 3)?;
            return if ev.eval(&args[0])?.truthy() {
                ev.eval(&args[1])
            } else {
                ev.eval(&args[2])
            };
        }
        "map" | "filter" | "find" | "any" | "all" | "sortBy" => return higher(ev, &a),
        "count" | "sum" | "avg" | "min" | "max" if args.len() == 2 && is_lambda(&args[1]) => {
            return higher(ev, &a);
        }
        _ => {}
    }
    if !FUNCTIONS.iter().any(|(n, _, _)| *n == name) {
        return Err(EvalError::new(
            ErrorKind::Function,
            format!("unknown function '{name}'"),
        ));
    }
    let mut v = Vec::with_capacity(args.len());
    for e in args {
        v.push(ev.eval(e)?);
    }
    plain(ev, &a, &v)
}

fn is_lambda(e: &Expr) -> bool {
    matches!(e, Expr::Lambda(..))
}

struct Args<'a> {
    name: &'a str,
    args: &'a [Expr],
}

impl Args<'_> {
    fn arity(&self, lo: usize, hi: usize) -> Result<(), EvalError> {
        let n = self.args.len();
        if n < lo || n > hi {
            let want = if lo == hi {
                lo.to_string()
            } else {
                format!("{lo} to {hi}")
            };
            return Err(EvalError::new(
                ErrorKind::Function,
                format!("{}() takes {want} arguments, got {n}", self.name),
            ));
        }
        Ok(())
    }
}

fn num(name: &str, v: &Value) -> Result<f64, EvalError> {
    v.as_num().ok_or_else(|| type_err(name, v))
}

fn list<'v>(name: &str, v: &'v Value) -> Result<&'v [Value], EvalError> {
    v.as_list().ok_or_else(|| type_err(name, v))
}

fn text(v: &Value) -> String {
    v.to_text()
}

fn date_of(name: &str, v: &Value) -> Result<i64, EvalError> {
    match v {
        Value::Date(d) => Ok(*d),
        Value::Str(s) => date::parse_iso(s).ok_or_else(|| type_err(name, v)),
        _ => Err(type_err(name, v)),
    }
}

fn opt_usize(v: Option<&Value>, default: usize) -> usize {
    v.and_then(Value::as_num)
        .map_or(default, |n| n.clamp(0.0, 10.0) as usize)
}

fn higher(ev: &mut Evaluator<'_>, a: &Args<'_>) -> EvalResult {
    a.arity(2, 3)?;
    let src = ev.eval(&a.args[0])?;
    if src.is_null() {
        return Ok(match a.name {
            "map" | "filter" | "sortBy" => Value::list(vec![]),
            "any" | "all" => Value::Bool(a.name == "all"),
            "count" | "sum" => Value::Num(0.0),
            _ => Value::Null,
        });
    }
    let items = list(a.name, &src)?.to_vec();
    let f = &a.args[1];
    let mut mapped = Vec::with_capacity(items.len());
    for (i, it) in items.iter().enumerate() {
        ev.tick(1)?;
        let r = ev.apply(f, &[it.clone(), Value::Num(i as f64)])?;
        match a.name {
            "find" if r.truthy() => return Ok(it.clone()),
            "any" if r.truthy() => return Ok(Value::Bool(true)),
            "all" if !r.truthy() => return Ok(Value::Bool(false)),
            _ => mapped.push(r),
        }
    }
    Ok(match a.name {
        "map" => Value::list(mapped),
        "filter" => Value::list(
            items
                .into_iter()
                .zip(&mapped)
                .filter(|(_, k)| k.truthy())
                .map(|(v, _)| v)
                .collect(),
        ),
        "find" => Value::Null,
        "any" => Value::Bool(false),
        "all" => Value::Bool(true),
        "count" => Value::Num(mapped.iter().filter(|v| v.truthy()).count() as f64),
        "sortBy" => {
            let desc = match a.args.get(2) {
                Some(e) => ev.eval(e)?.truthy(),
                None => false,
            };
            ev.tick(items.len() as u64)?;
            let mut pairs: Vec<_> = mapped.into_iter().zip(items).collect();
            pairs.sort_by(|x, y| {
                let o = compare(&x.0, &y.0).unwrap_or(std::cmp::Ordering::Equal);
                if desc { o.reverse() } else { o }
            });
            Value::list(pairs.into_iter().map(|(_, v)| v).collect())
        }
        _ => aggregate(a.name, &mapped)?,
    })
}

fn aggregate(name: &str, items: &[Value]) -> EvalResult {
    let nums: Vec<f64> = items.iter().filter_map(Value::as_num).collect();
    Ok(match name {
        "count" => Value::Num(items.len() as f64),
        "sum" => Value::Num(nums.iter().sum()),
        "avg" if nums.is_empty() => Value::Null,
        "avg" => Value::Num(nums.iter().sum::<f64>() / nums.len() as f64),
        "min" | "max" => {
            let mut best: Option<&Value> = None;
            for v in items.iter().filter(|v| !v.is_null()) {
                best = match best {
                    None => Some(v),
                    Some(b) => {
                        let o = compare(v, b).ok_or_else(|| type_err(name, v))?;
                        if (name == "min" && o.is_lt()) || (name == "max" && o.is_gt()) {
                            Some(v)
                        } else {
                            Some(b)
                        }
                    }
                };
            }
            best.cloned().unwrap_or_default()
        }
        _ => Value::Null,
    })
}

fn plain(ev: &mut Evaluator<'_>, a: &Args<'_>, v: &[Value]) -> EvalResult {
    let n = a.name;
    let one = |lo, hi| a.arity(lo, hi);
    // Missing input gives a missing result rather than an error, so partial views render.
    let null_in = v.first().is_some_and(Value::is_null)
        && !matches!(
            n,
            "coalesce" | "text" | "len" | "date" | "count" | "sum" | "plural" | "join"
        );
    if null_in {
        return Ok(Value::Null);
    }
    Ok(match n {
        "abs" => {
            one(1, 1)?;
            Value::Num(num(n, &v[0])?.abs())
        }
        "floor" => {
            one(1, 1)?;
            Value::Num(num(n, &v[0])?.floor())
        }
        "ceil" => {
            one(1, 1)?;
            Value::Num(num(n, &v[0])?.ceil())
        }
        "sqrt" => {
            one(1, 1)?;
            Value::Num(num(n, &v[0])?.sqrt())
        }
        "round" => {
            one(1, 2)?;
            let p = 10f64.powi(opt_usize(v.get(1), 0) as i32);
            Value::Num((num(n, &v[0])? * p).round() / p)
        }
        "pow" => {
            one(2, 2)?;
            Value::Num(num(n, &v[0])?.powf(num(n, &v[1])?))
        }
        "clamp" => {
            one(3, 3)?;
            let (x, lo, hi) = (num(n, &v[0])?, num(n, &v[1])?, num(n, &v[2])?);
            Value::Num(x.max(lo).min(hi))
        }
        "min" | "max" | "sum" | "avg" | "count" => {
            if v.len() == 1 {
                if v[0].is_null() {
                    return aggregate(n, &[]);
                }
                ev.tick(list(n, &v[0])?.len() as u64)?;
                aggregate(n, list(n, &v[0])?)?
            } else {
                one(1, 64)?;
                aggregate(n, v)?
            }
        }
        "len" => {
            one(1, 1)?;
            match &v[0] {
                Value::Str(s) => Value::Num(s.chars().count() as f64),
                Value::List(l) => Value::Num(l.len() as f64),
                Value::Map(m) => Value::Num(m.len() as f64),
                Value::Null => Value::Num(0.0),
                other => return Err(type_err(n, other)),
            }
        }
        "upper" => {
            one(1, 1)?;
            Value::str(text(&v[0]).to_uppercase())
        }
        "lower" => {
            one(1, 1)?;
            Value::str(text(&v[0]).to_lowercase())
        }
        "trim" => {
            one(1, 1)?;
            Value::str(text(&v[0]).trim())
        }
        "text" => {
            one(1, 1)?;
            Value::str(text(&v[0]))
        }
        "contains" => {
            one(2, 2)?;
            match &v[0] {
                Value::List(l) => {
                    ev.tick(l.len() as u64)?;
                    Value::Bool(l.contains(&v[1]))
                }
                other => Value::Bool(text(other).contains(&text(&v[1]))),
            }
        }
        "startsWith" => {
            one(2, 2)?;
            Value::Bool(text(&v[0]).starts_with(&text(&v[1])))
        }
        "replace" => {
            one(3, 3)?;
            let find = text(&v[1]);
            if find.is_empty() {
                return Ok(v[0].clone());
            }
            let s = text(&v[0]).replace(&find, &text(&v[2]));
            ev.check_text(s.len())?;
            Value::str(s)
        }
        "split" => {
            one(2, 2)?;
            let sep = text(&v[1]);
            let s = text(&v[0]);
            let parts: Vec<Value> = if sep.is_empty() {
                s.chars().map(|c| Value::str(c.to_string())).collect()
            } else {
                s.split(sep.as_str()).map(Value::str).collect()
            };
            ev.check_list(parts.len())?;
            Value::list(parts)
        }
        "join" => {
            one(1, 2)?;
            let sep = v.get(1).map_or(", ".to_string(), text);
            let items = if v[0].is_null() {
                &[][..]
            } else {
                list(n, &v[0])?
            };
            let s = items.iter().map(text).collect::<Vec<_>>().join(&sep);
            ev.check_text(s.len())?;
            Value::str(s)
        }
        "plural" => {
            one(3, 3)?;
            let k = v[0].as_num().unwrap_or(0.0);
            let word = if k == 1.0 { text(&v[1]) } else { text(&v[2]) };
            Value::str(format!("{} {word}", crate::value::fmt_num(k)))
        }
        "reverse" => {
            one(1, 1)?;
            let mut l = list(n, &v[0])?.to_vec();
            l.reverse();
            Value::list(l)
        }
        "first" => {
            one(1, 1)?;
            list(n, &v[0])?.first().cloned().unwrap_or_default()
        }
        "last" => {
            one(1, 1)?;
            list(n, &v[0])?.last().cloned().unwrap_or_default()
        }
        "slice" => {
            one(2, 3)?;
            let start = num(n, &v[1])?.max(0.0) as usize;
            match &v[0] {
                Value::Str(s) => {
                    let chars: Vec<char> = s.chars().collect();
                    let end = v
                        .get(2)
                        .and_then(Value::as_num)
                        .map_or(chars.len(), |e| e.max(0.0) as usize);
                    Value::str(
                        chars[start.min(chars.len())
                            ..end.clamp(start.min(chars.len()), chars.len())]
                            .iter()
                            .collect::<String>(),
                    )
                }
                other => {
                    let l = list(n, other)?;
                    let end = v
                        .get(2)
                        .and_then(Value::as_num)
                        .map_or(l.len(), |e| e.max(0.0) as usize);
                    let s = start.min(l.len());
                    Value::list(l[s..end.clamp(s, l.len())].to_vec())
                }
            }
        }
        "range" => {
            one(1, 2)?;
            let (lo, hi) = if v.len() == 1 {
                (0.0, num(n, &v[0])?)
            } else {
                (num(n, &v[0])?, num(n, &v[1])?)
            };
            let len = (hi - lo).ceil().max(0.0);
            if len > ev.limits.max_list as f64 {
                return Err(EvalError::new(
                    ErrorKind::Size,
                    "range is longer than the list limit",
                ));
            }
            ev.tick(len as u64)?;
            Value::list(
                (0..len as usize)
                    .map(|i| Value::Num(lo + i as f64))
                    .collect(),
            )
        }
        "unique" => {
            one(1, 1)?;
            let l = list(n, &v[0])?;
            ev.tick((l.len() * l.len()) as u64 / 8 + 1)?;
            let mut out: Vec<Value> = Vec::new();
            for it in l {
                if !out.contains(it) {
                    out.push(it.clone());
                }
            }
            Value::list(out)
        }
        "keys" | "values" => {
            one(1, 1)?;
            let Value::Map(m) = &v[0] else {
                return Err(type_err(n, &v[0]));
            };
            if n == "keys" {
                Value::list(m.keys().map(Value::str).collect())
            } else {
                Value::list(m.values().cloned().collect())
            }
        }
        "merge" => {
            one(2, 2)?;
            match (&v[0], &v[1]) {
                (Value::Map(x), Value::Map(y)) => {
                    let mut m = (**x).clone();
                    m.extend(y.iter().map(|(k, v)| (k.clone(), v.clone())));
                    Value::map(m)
                }
                (_, other) => return Err(type_err(n, other)),
            }
        }
        "date" => {
            one(1, 3)?;
            if v.len() == 3 {
                let (y, m, d) = (
                    num(n, &v[0])? as i64,
                    num(n, &v[1])? as u32,
                    num(n, &v[2])? as u32,
                );
                if !(1..=12).contains(&m) || d == 0 || d > date::days_in_month(y, m) {
                    return Ok(Value::Null);
                }
                Value::Date(date::from_civil(y, m, d))
            } else {
                match &v[0] {
                    Value::Date(d) => Value::Date(*d),
                    Value::Str(s) => date::parse_iso(s).map_or(Value::Null, Value::Date),
                    _ => Value::Null,
                }
            }
        }
        "year" => {
            one(1, 1)?;
            Value::Num(date::to_civil(date_of(n, &v[0])?).0 as f64)
        }
        "month" => {
            one(1, 1)?;
            Value::Num(date::to_civil(date_of(n, &v[0])?).1 as f64)
        }
        "day" => {
            one(1, 1)?;
            Value::Num(date::to_civil(date_of(n, &v[0])?).2 as f64)
        }
        "weekday" => {
            one(1, 1)?;
            Value::Num(date::weekday(date_of(n, &v[0])?) as f64)
        }
        "addDays" => {
            one(2, 2)?;
            Value::Date(date_of(n, &v[0])? + num(n, &v[1])? as i64)
        }
        "addMonths" => {
            one(2, 2)?;
            Value::Date(date::add_months(date_of(n, &v[0])?, num(n, &v[1])? as i64))
        }
        "daysBetween" => {
            one(2, 2)?;
            Value::Num((date_of(n, &v[1])? - date_of(n, &v[0])?) as f64)
        }
        "number" => {
            one(1, 2)?;
            Value::str(format::grouped(num(n, &v[0])?, opt_usize(v.get(1), 0)))
        }
        "currency" => {
            one(1, 3)?;
            let code = v.get(1).map_or("USD".to_string(), text);
            let dec = v.get(2).map(|d| opt_usize(Some(d), 2));
            Value::str(format::currency(num(n, &v[0])?, &code, dec))
        }
        "percent" => {
            one(1, 2)?;
            Value::str(format::percent(num(n, &v[0])?, opt_usize(v.get(1), 0)))
        }
        "compact" => {
            one(1, 1)?;
            Value::str(format::compact(num(n, &v[0])?))
        }
        "formatDate" => {
            one(1, 2)?;
            let pat = v.get(1).map_or("MMM d, yyyy".to_string(), text);
            Value::str(date::format(date_of(n, &v[0])?, &pat))
        }
        "coalesce" => v.iter().find(|x| !x.is_null()).cloned().unwrap_or_default(),
        "num" => {
            one(1, 1)?;
            match &v[0] {
                Value::Num(x) => Value::Num(*x),
                Value::Bool(b) => Value::Num(if *b { 1.0 } else { 0.0 }),
                Value::Str(s) => s
                    .trim()
                    .replace(',', "")
                    .parse::<f64>()
                    .map_or(Value::Null, Value::Num),
                _ => Value::Null,
            }
        }
        _ => {
            return Err(EvalError::new(
                ErrorKind::Function,
                format!("unknown function '{n}'"),
            ));
        }
    })
}
