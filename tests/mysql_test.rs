use chain_builder::{ChainBuilder, Client, JoinMethods, Select, WhereClauses};
use serde_json::{self, Value};

#[test]
fn test_chain_builder() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder.db("mydb"); // For dynamic db
    builder.select(Select::Columns(vec!["*".into()]));
    builder.table("users");
    builder.query(|qb| {
        qb.where_eq("name", serde_json::Value::String("John".to_string()));
        qb.where_eq("city", serde_json::Value::String("New York".to_string()));
        qb.where_in(
            "department",
            vec![
                serde_json::Value::String("IT".to_string()),
                serde_json::Value::String("HR".to_string()),
            ],
        );

        qb.where_subquery(|sub| {
            sub.where_eq("status", serde_json::Value::String("active".to_string()));
            sub.or()
                .where_eq("status", serde_json::Value::String("pending".to_string()))
                .where_between(
                    "registered_at",
                    [
                        serde_json::Value::String("2024-01-01".to_string()),
                        serde_json::Value::String("2024-01-31".to_string()),
                    ],
                );
        });

        qb.where_raw(
            "(latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?)".into(),
            Some(vec![
                Value::Number(serde_json::Number::from_f64(40.0).unwrap()),
                Value::Number(serde_json::Number::from_f64(41.0).unwrap()),
                Value::Number(serde_json::Number::from_f64(70.0).unwrap()),
                Value::Number(serde_json::Number::from_f64(71.0).unwrap()),
            ]),
        );
    });

    builder.add_raw("LIMIT ?".into(), Some(vec![10.into()]));

    let sql = builder.to_sql();
    // println!("final sql: {}", sql.0);
    // println!("final binds: {:?}", sql.1);
    assert_eq!(
            sql.0,
            "SELECT * FROM `mydb`.`users` WHERE name = ? AND city = ? AND department IN (?,?) AND (status = ? OR (status = ? AND registered_at BETWEEN ? AND ?)) AND (latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?) LIMIT ?"
        );
    assert_eq!(
        sql.1,
        vec![
            Value::String("John".to_string()),
            Value::String("New York".to_string()),
            Value::String("IT".to_string()),
            Value::String("HR".to_string()),
            Value::String("active".to_string()),
            Value::String("pending".to_string()),
            Value::String("2024-01-01".to_string()),
            Value::String("2024-01-31".to_string()),
            Value::Number(serde_json::Number::from_f64(40.0).unwrap()),
            Value::Number(serde_json::Number::from_f64(41.0).unwrap()),
            Value::Number(serde_json::Number::from_f64(70.0).unwrap()),
            Value::Number(serde_json::Number::from_f64(71.0).unwrap()),
            Value::Number(10.into()),
        ]
    );
}

#[test]
fn test_join() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder.db("mydb"); // For dynamic db
    builder.select(Select::Columns(vec!["*".into()]));
    builder.table("users");
    builder.query(|qb| {
        qb.join("details", |join| {
            join.on("details.id", "=", "users.d_id");
            join.on("details.id_w", "=", "users.d_id_w");
            join.or().on("details.id_s", "=", "users.d_id_s").on(
                "details.id_w",
                "=",
                "users.d_id_w",
            );
        });
        qb.where_eq("name", serde_json::Value::String("John".to_string()));
    });
    builder.select(Select::Raw(
        "(SELECT COUNT(*) FROM `mydb`.`users` WHERE users.id = ?) AS count".into(),
        Some(vec![serde_json::Value::Number(1.into())]),
    ));
    let sql = builder.to_sql();
    // println!("final sql: {}", sql.0);
    // println!("final binds: {:?}", sql.1);
    assert_eq!(
            sql.0,
            "SELECT *, (SELECT COUNT(*) FROM `mydb`.`users` WHERE users.id = ?) AS count FROM `mydb`.`users` JOIN `mydb`.`details` ON details.id = users.d_id AND details.id_w = users.d_id_w OR (details.id_s = users.d_id_s AND details.id_w = users.d_id_w) WHERE name = ?"
        );
    assert_eq!(
        sql.1,
        vec![
            serde_json::Value::Number(1.into()),
            serde_json::Value::String("John".to_string()),
        ]
    );
}


#[test]
fn test_insert() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder.db("mydb"); // For dynamic db
    builder.table("users");
    builder.insert(serde_json::json!({
        "name": "John",
        "`city`": "New York",
        "department": "IT",
    }));
    let sql = builder.to_sql();
    println!("final sql: {}", sql.0);
    println!("final binds: {:?}", sql.1);
    assert_eq!(
            sql.0,
            "INSERT INTO `mydb`.`users` (`city`, department, name) VALUES (?, ?, ?)"
        );
    assert_eq!(
        sql.1,
        vec![
            serde_json::Value::String("New York".to_string()),
            serde_json::Value::String("IT".to_string()),
            serde_json::Value::String("John".to_string())
        ]
    );
}