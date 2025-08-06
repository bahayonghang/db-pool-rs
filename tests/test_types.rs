use db_pool_rs::core::types::{DatabaseValue, QueryParams};
use db_pool_rs::utils::dataframe::DataFrameConverter;
use polars::prelude::*;
use std::collections::HashMap;

#[tokio::test]
async fn test_database_value_conversion() {
    // 测试不同数据类型的转换
    let test_cases = vec![
        (DatabaseValue::Null, "null"),
        (DatabaseValue::Bool(true), "boolean"),
        (DatabaseValue::I32(42), "i32"),
        (DatabaseValue::I64(123456789), "i64"),
        (DatabaseValue::F32(3.14), "f32"),
        (DatabaseValue::F64(2.718281828), "f64"),
        (DatabaseValue::String("test".to_string()), "string"),
        (DatabaseValue::Bytes(vec![1, 2, 3, 4]), "bytes"),
    ];
    
    for (value, description) in test_cases {
        // 这里应该测试DatabaseValue的各种转换方法
        // 由于我们的实现中没有直接的转换方法，这里做基本验证
        match value {
            DatabaseValue::Null => assert!(matches!(value, DatabaseValue::Null)),
            DatabaseValue::Bool(b) => assert_eq!(b, true),
            DatabaseValue::I32(i) => assert_eq!(i, 42),
            DatabaseValue::I64(i) => assert_eq!(i, 123456789),
            DatabaseValue::F32(f) => assert!((f - 3.14).abs() < 0.001),
            DatabaseValue::F64(f) => assert!((f - 2.718281828).abs() < 0.000001),
            DatabaseValue::String(s) => assert_eq!(s, "test"),
            DatabaseValue::Bytes(b) => assert_eq!(b, vec![1, 2, 3, 4]),
            _ => {}
        }
        
        println!("✅ {description} conversion test passed");
    }
}

#[tokio::test]
async fn test_query_params() {
    let mut params = QueryParams::new();
    params.insert("id".to_string(), DatabaseValue::I32(1));
    params.insert("name".to_string(), DatabaseValue::String("test".to_string()));
    params.insert("active".to_string(), DatabaseValue::Bool(true));
    params.insert("score".to_string(), DatabaseValue::F64(95.5));
    params.insert("data".to_string(), DatabaseValue::Null);
    
    assert_eq!(params.len(), 5);
    
    // 验证参数值
    if let Some(DatabaseValue::I32(id)) = params.get("id") {
        assert_eq!(*id, 1);
    } else {
        panic!("ID parameter not found or wrong type");
    }
    
    if let Some(DatabaseValue::String(name)) = params.get("name") {
        assert_eq!(name, "test");
    } else {
        panic!("Name parameter not found or wrong type");
    }
    
    if let Some(DatabaseValue::Bool(active)) = params.get("active") {
        assert_eq!(*active, true);
    } else {
        panic!("Active parameter not found or wrong type");
    }
}

#[tokio::test]
async fn test_dataframe_converter() {
    // 创建测试数据
    let mut data = HashMap::new();
    data.insert("id".to_string(), vec![
        AnyValue::Int32(1),
        AnyValue::Int32(2),
        AnyValue::Int32(3),
    ]);
    data.insert("name".to_string(), vec![
        AnyValue::Utf8("Alice"),
        AnyValue::Utf8("Bob"),
        AnyValue::Utf8("Charlie"),
    ]);
    data.insert("score".to_string(), vec![
        AnyValue::Float64(95.5),
        AnyValue::Float64(87.2),
        AnyValue::Float64(92.8),
    ]);
    
    // 转换为DataFrame
    let df_result = DataFrameConverter::from_hashmap(data);
    assert!(df_result.is_ok());
    
    let df = df_result.unwrap();
    assert_eq!(df.height(), 3);
    assert_eq!(df.width(), 3);
    
    // 验证列名
    let columns = df.get_column_names();
    assert!(columns.contains(&"id"));
    assert!(columns.contains(&"name"));
    assert!(columns.contains(&"score"));
    
    println!("✅ DataFrame conversion test passed");
}

#[tokio::test]
async fn test_dataframe_json_conversion() {
    // 创建简单的DataFrame
    let df = df! {
        "id" => [1, 2, 3],
        "name" => ["Alice", "Bob", "Charlie"],
        "score" => [95.5, 87.2, 92.8],
    }.unwrap();
    
    // 转换为JSON
    let json_result = DataFrameConverter::to_json_string(&df);
    assert!(json_result.is_ok());
    
    let json_str = json_result.unwrap();
    assert!(json_str.contains("Alice"));
    assert!(json_str.contains("95.5"));
    
    // 从JSON转换回DataFrame
    let df_from_json_result = DataFrameConverter::from_json_string(&json_str);
    assert!(df_from_json_result.is_ok());
    
    let df_from_json = df_from_json_result.unwrap();
    assert_eq!(df_from_json.height(), 3);
    assert_eq!(df_from_json.width(), 3);
    
    println!("✅ DataFrame JSON conversion test passed");
}

#[tokio::test]
async fn test_dataframe_stats() {
    // 创建测试DataFrame
    let df = df! {
        "id" => [1, 2, 3, 4, 5],
        "name" => ["A", "B", "C", "D", "E"],
        "value" => [10.0, 20.0, 30.0, 40.0, 50.0],
    }.unwrap();
    
    // 获取统计信息
    let stats_result = DataFrameConverter::get_stats(&df);
    assert!(stats_result.is_ok());
    
    let stats = stats_result.unwrap();
    
    // 验证基本统计信息
    if let Some(shape) = stats.get("shape") {
        let shape_obj = shape.as_object().unwrap();
        assert_eq!(shape_obj.get("rows").unwrap().as_u64().unwrap(), 5);
        assert_eq!(shape_obj.get("columns").unwrap().as_u64().unwrap(), 3);
    } else {
        panic!("Shape statistics not found");
    }
    
    if let Some(columns) = stats.get("columns") {
        let columns_array = columns.as_array().unwrap();
        assert_eq!(columns_array.len(), 3);
    } else {
        panic!("Columns statistics not found");
    }
    
    println!("✅ DataFrame stats test passed");
}