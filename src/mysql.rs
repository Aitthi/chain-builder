use crate::join::JoinStatement;
use crate::{operator::Operator, ChainBuilder, Method, Select, Statement};

// operator and is_bind
pub fn operator_to_sql(operator: &Operator) -> (&str, bool) {
    match operator {
        Operator::Equal => ("=", true),
        Operator::NotEqual => ("!=", true),
        Operator::In => ("IN", true),
        Operator::NotIn => ("NOT IN", true),
        Operator::IsNull => ("IS NULL", false),
        Operator::IsNotNull => ("IS NOT NULL", false),
        Operator::Exists => ("EXISTS", false),
        Operator::NotExists => ("NOT EXISTS", false),
        Operator::Between => ("BETWEEN", true),
        Operator::NotBetween => ("NOT BETWEEN", true),
        Operator::Like => ("LIKE", true),
        Operator::NotLike => ("NOT LIKE", true),
    }
}

pub struct ToSql {
    pub sql: String,
    pub binds: Option<Vec<serde_json::Value>>,
    pub select_binds: Option<Vec<serde_json::Value>>,
    pub join_binds: Option<Vec<serde_json::Value>>,
}
pub fn to_sql(c: &ChainBuilder, is_statement: bool) -> ToSql {
    let mut statement_sql = String::new();
    let mut to_binds: Vec<serde_json::Value> = vec![];
    let mut to_select_binds: Vec<serde_json::Value> = vec![];
    let mut to_join_binds: Vec<serde_json::Value> = vec![];
    for (i, statement) in c.query.statement.iter().enumerate() {
        match statement {
            Statement::Value(field, operator, value) => {
                if i > 0 {
                    if let Some(s) = c.query.statement.get(i - 1) {
                        match s {
                            Statement::OrChain(_) => {}
                            _ => {
                                statement_sql.push_str(" AND ");
                            }
                        }
                    }
                }
                statement_sql.push_str(&format!("{} ", field));
                let (operator_str, is_bind) = operator_to_sql(operator);
                if *operator == Operator::Between || *operator == Operator::NotBetween {
                    statement_sql.push_str(&format!("{} ? AND ?", operator_str));
                    for v in value.as_array().unwrap() {
                        to_binds.push(v.clone());
                    }
                } else {
                    statement_sql.push_str(operator_str);
                    if is_bind {
                        if let serde_json::Value::Array(value) = value {
                            statement_sql.push_str(" (");
                            let mut is_first = true;
                            value.iter().for_each(|v| {
                                if is_first {
                                    is_first = false;
                                } else {
                                    statement_sql.push(',');
                                }
                                statement_sql.push('?');
                                to_binds.push(v.clone());
                            });
                            statement_sql.push(')');
                        } else {
                            statement_sql.push_str(" ?");
                            to_binds.push(value.clone());
                        }
                    }
                }
            }
            Statement::OrChain(qb) => {
                if i > 0 {
                    statement_sql.push_str(" OR ");
                }
                let mut c = c.clone();
                c.query = *qb.clone();
                let rs_tosql = to_sql(&c, true);
                if qb.statement.len() > 1 {
                    statement_sql.push_str(&format!("({})", rs_tosql.sql));
                } else {
                    statement_sql.push_str(&rs_tosql.sql);
                }
                if let Some(binds) = rs_tosql.binds {
                    to_binds.extend(binds);
                }
                if let Some(select_binds) = rs_tosql.select_binds {
                    to_select_binds.extend(select_binds);
                }
                if let Some(join_binds) = rs_tosql.join_binds {
                    to_join_binds.extend(join_binds);
                }
            }
            Statement::SubChain(qb) => {
                if i > 0 {
                    statement_sql.push_str(" AND ");
                }
                let mut c = c.clone();
                c.query = *qb.clone();
                let rs_tosql = to_sql(&c, true);
                statement_sql.push_str(&format!("({})", rs_tosql.sql));
                if let Some(binds) = rs_tosql.binds {
                    to_binds.extend(binds);
                }
                if let Some(select_binds) = rs_tosql.select_binds {
                    to_select_binds.extend(select_binds);
                }
                if let Some(join_binds) = rs_tosql.join_binds {
                    to_join_binds.extend(join_binds);
                }
            }
            Statement::Raw((sql, binds)) => {
                if i > 0 {
                    statement_sql.push_str(" AND ");
                }
                statement_sql.push_str(sql);
                if let Some(binds) = binds {
                    to_binds.extend(binds.clone());
                }
            }
        }
    }

    if is_statement {
        return ToSql {
            sql: statement_sql,
            binds: Some(to_binds),
            select_binds: Some(to_select_binds),
            join_binds: Some(to_join_binds),
        };
    }

    let mut to_sql_str = String::new();
    if c.method == Method::Select {
        to_sql_str.push_str("SELECT ");
    }

    if c.select.is_empty() {
        to_sql_str.push('*');
    } else {
        let mut is_first = true;
        for select in &c.select {
            if is_first {
                is_first = false;
            } else {
                to_sql_str.push_str(", ");
            }
            match select {
                Select::Columns(columns) => {
                    to_sql_str.push_str(&columns.join(", "));
                }
                Select::Raw((sql, binds)) => {
                    to_sql_str.push_str(sql);
                    if let Some(binds) = binds {
                        to_select_binds.extend(binds.clone());
                    }
                }
                Select::Builder(subc) => {
                    let rs_tosql = to_sql(&subc.1, false);
                    to_sql_str.push_str(&format!("({}) AS {}", rs_tosql.sql, subc.0));

                    // Add all binds to select_binds order by select_binds, join_binds, binds
                    if let Some(select_binds) = rs_tosql.select_binds {
                        // 1. add select_binds
                        to_select_binds.extend(select_binds);
                    }
                    if let Some(join_binds) = rs_tosql.join_binds {
                        // 2. add join_binds
                        to_select_binds.extend(join_binds);
                    }
                    if let Some(binds) = rs_tosql.binds {
                        // 3. add binds
                        to_select_binds.extend(binds.clone());
                    }
                }
            }
        }
    }

    to_sql_str.push_str(" FROM ");
    if let Some(db) = &c.db {
        to_sql_str.push_str(db);
        to_sql_str.push('.');
    }
    to_sql_str.push_str(&c.table);
    if let Some(as_name) = &c.as_name {
        to_sql_str.push_str(" AS ");
        to_sql_str.push_str(as_name);
    }
    if !c.query.join.is_empty() {
        let (sql, binds) = join_to_sql(c, true);
        to_sql_str.push_str(&format!(" {}", sql));
        if let Some(binds) = binds {
            to_join_binds.extend(binds.clone());
        }
    }
    if !statement_sql.is_empty() {
        to_sql_str.push_str(" WHERE ");
        to_sql_str.push_str(&statement_sql);
    }
    // if let Some(raw) = &c.query.raw {
    //     to_sql_str.push(' ');
    //     to_sql_str.push_str(&raw.0);
    //     if let Some(binds) = &raw.1 {
    //         to_binds.extend(binds.clone());
    //     }
    // }
    if !c.query.raw.is_empty() {
        for raw in &c.query.raw {
            to_sql_str.push(' ');
            to_sql_str.push_str(&raw.0);
            if let Some(binds) = &raw.1 {
                to_binds.extend(binds.clone());
            }
        }
    }
    // (to_sql_str, Some(to_binds))
    ToSql {
        sql: to_sql_str,
        binds: Some(to_binds),
        select_binds: Some(to_select_binds),
        join_binds: Some(to_join_binds),
    }
}

fn join_to_sql(c: &ChainBuilder, prefix: bool) -> (String, Option<Vec<serde_json::Value>>) {
    let mut to_sql_str = String::new();
    let mut to_binds: Vec<serde_json::Value> = vec![];
    for (i, join) in c.query.join.iter().enumerate() {
        if i > 0 {
            to_sql_str.push(' ');
        }

        if prefix {
            let table = if let Some(db) = &c.db {
                format!("{}.{}", db, join.table)
            } else {
                join.table.clone()
            };
            to_sql_str.push_str(&format!("{} {} ON ", join.join_type, table));
        }

        for (j, statement) in join.statement.iter().enumerate() {
            match statement {
                JoinStatement::On(column, operator, column2) => {
                    if j > 0 {
                        to_sql_str.push_str(" AND ");
                    }
                    to_sql_str.push_str(format!("{} {} {}", column, operator, column2).as_str());
                }
                JoinStatement::OrChain(qb) => {
                    if j > 0 {
                        to_sql_str.push_str(" OR ");
                    }
                    let mut c = c.clone();
                    c.query.join = vec![*qb.clone()];
                    let (sql, binds) = join_to_sql(&c, false);
                    to_sql_str.push_str(&format!("({})", sql));
                    if let Some(binds) = binds {
                        to_binds.extend(binds);
                    }
                }
                JoinStatement::OnVal(column, operator, value) => {
                    if j > 0 {
                        to_sql_str.push_str(" AND ");
                    }
                    to_sql_str.push_str(format!("{} {} ?", column, operator).as_str());
                    to_binds.push(value.clone());
                }
            }
        }
    }
    (to_sql_str, Some(to_binds))
}
