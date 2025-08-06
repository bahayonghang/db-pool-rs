use crate::core::error::{ConversionError, Result};
use polars::prelude::*;
use std::collections::HashMap;

/// DataFrame转换工具
pub struct DataFrameConverter;

impl DataFrameConverter {
    /// 将HashMap转换为DataFrame
    pub fn from_hashmap(data: HashMap<String, Vec<AnyValue>>) -> Result<DataFrame> {
        if data.is_empty() {
            return Ok(DataFrame::empty());
        }

        let mut series_vec = Vec::new();
        
        for (column_name, values) in data {
            let series = Self::create_series_from_any_values(&column_name, values)?;
            series_vec.push(series);
        }

        DataFrame::new(series_vec)
            .map_err(|e| ConversionError::DataFrameConversion(e.to_string()).into())
    }

    /// 将DataFrame转换为JSON字符串
    pub fn to_json_string(df: &DataFrame) -> Result<String> {
        let mut json_rows = Vec::new();
        let column_names = df.get_column_names();

        for row_idx in 0..df.height() {
            let mut row_map = serde_json::Map::new();
            
            for col_name in &column_names {
                let series = df.column(col_name)
                    .map_err(|e| ConversionError::DataFrameConversion(e.to_string()))?;
                
                let value = series.get(row_idx)
                    .map_err(|e| ConversionError::DataFrameConversion(e.to_string()))?;
                
                let json_value = Self::any_value_to_json_value(value)?;
                row_map.insert(col_name.to_string(), json_value);
            }
            
            json_rows.push(serde_json::Value::Object(row_map));
        }

        serde_json::to_string(&json_rows)
            .map_err(|e| ConversionError::Serialization(e.to_string()).into())
    }

    /// 从JSON字符串创建DataFrame
    pub fn from_json_string(json_str: &str) -> Result<DataFrame> {
        let json_value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| ConversionError::Deserialization(e.to_string()))?;

        match json_value {
            serde_json::Value::Array(rows) => {
                if rows.is_empty() {
                    return Ok(DataFrame::empty());
                }

                // 收集所有列名
                let mut all_columns = std::collections::HashSet::new();
                for row in &rows {
                    if let serde_json::Value::Object(obj) = row {
                        for key in obj.keys() {
                            all_columns.insert(key.clone());
                        }
                    }
                }

                let column_names: Vec<String> = all_columns.into_iter().collect();
                let mut column_data: HashMap<String, Vec<AnyValue>> = HashMap::new();

                // 初始化列数据
                for col_name in &column_names {
                    column_data.insert(col_name.clone(), Vec::new());
                }

                // 填充数据
                for row in rows {
                    if let serde_json::Value::Object(obj) = row {
                        for col_name in &column_names {
                            let value = obj.get(col_name)
                                .cloned()  // 克隆值避免借用问题
                                .unwrap_or(serde_json::Value::Null);
                            let any_value = Self::json_value_to_any_value(value)?;
                            column_data.get_mut(col_name).unwrap().push(any_value);
                        }
                    }
                }

                Self::from_hashmap(column_data)
            }
            _ => Err(ConversionError::Deserialization(
                "JSON必须是数组格式".to_string()
            ).into()),
        }
    }

    /// 获取DataFrame的基本统计信息
    pub fn get_stats(df: &DataFrame) -> Result<HashMap<String, serde_json::Value>> {
        let mut stats = HashMap::new();
        
        stats.insert("shape".to_string(), serde_json::json!({
            "rows": df.height(),
            "columns": df.width()
        }));

        stats.insert("columns".to_string(), serde_json::json!(df.get_column_names()));

        // 数据类型
        let mut dtypes = HashMap::new();
        for col_name in df.get_column_names() {
            if let Ok(series) = df.column(col_name) {
                dtypes.insert(col_name.to_string(), format!("{:?}", series.dtype()));
            }
        }
        stats.insert("dtypes".to_string(), serde_json::json!(dtypes));

        // 内存使用
        let memory_usage = df.estimated_size();
        stats.insert("memory_usage_bytes".to_string(), serde_json::json!(memory_usage));

        Ok(stats)
    }

    // 私有辅助方法

    /// 从AnyValue向量创建Series
    fn create_series_from_any_values(name: &str, values: Vec<AnyValue>) -> Result<Series> {
        if values.is_empty() {
            return Ok(Series::new_empty(name, &DataType::Null));
        }

        // 推断数据类型
        let data_type = Self::infer_data_type(&values);

        match data_type {
            DataType::Boolean => {
                let bool_values: Vec<Option<bool>> = values
                    .into_iter()
                    .map(|v| match v {
                        AnyValue::Boolean(b) => Some(b),
                        AnyValue::Null => None,
                        _ => None,
                    })
                    .collect();
                Ok(Series::new(name, bool_values))
            }
            DataType::Int32 => {
                let int_values: Vec<Option<i32>> = values
                    .into_iter()
                    .map(|v| match v {
                        AnyValue::Int32(i) => Some(i),
                        AnyValue::Null => None,
                        _ => None,
                    })
                    .collect();
                Ok(Series::new(name, int_values))
            }
            DataType::Int64 => {
                let int_values: Vec<Option<i64>> = values
                    .into_iter()
                    .map(|v| match v {
                        AnyValue::Int64(i) => Some(i),
                        AnyValue::Int32(i) => Some(i as i64),
                        AnyValue::Null => None,
                        _ => None,
                    })
                    .collect();
                Ok(Series::new(name, int_values))
            }
            DataType::Float64 => {
                let float_values: Vec<Option<f64>> = values
                    .into_iter()
                    .map(|v| match v {
                        AnyValue::Float64(f) => Some(f),
                        AnyValue::Float32(f) => Some(f as f64),
                        AnyValue::Int32(i) => Some(i as f64),
                        AnyValue::Int64(i) => Some(i as f64),
                        AnyValue::Null => None,
                        _ => None,
                    })
                    .collect();
                Ok(Series::new(name, float_values))
            }
            DataType::String => {
                let string_values: Vec<Option<String>> = values
                    .into_iter()
                    .map(|v| match v {
                        AnyValue::String(s) => Some(s.to_string()),
                        AnyValue::StringOwned(s) => Some(s.to_string()),
                        AnyValue::Null => None,
                        _ => Some(format!("{:?}", v)),
                    })
                    .collect();
                Ok(Series::new(name, string_values))
            }
            _ => {
                // 默认转换为字符串
                let string_values: Vec<Option<String>> = values
                    .into_iter()
                    .map(|v| match v {
                        AnyValue::Null => None,
                        _ => Some(format!("{:?}", v)),
                    })
                    .collect();
                Ok(Series::new(name, string_values))
            }
        }
    }

    /// 推断数据类型
    fn infer_data_type(values: &[AnyValue]) -> DataType {
        for value in values {
            match value {
                AnyValue::Boolean(_) => return DataType::Boolean,
                AnyValue::Int32(_) => return DataType::Int32,
                AnyValue::Int64(_) => return DataType::Int64,
                AnyValue::Float32(_) | AnyValue::Float64(_) => return DataType::Float64,
                AnyValue::String(_) | AnyValue::StringOwned(_) => return DataType::String,
                AnyValue::Datetime(_, time_unit, _) => {
                    return DataType::Datetime(*time_unit, None);
                }
                AnyValue::Binary(_) | AnyValue::BinaryOwned(_) => return DataType::Binary,
                AnyValue::Null => continue,
                _ => return DataType::String,
            }
        }
        DataType::Null
    }

    /// 将AnyValue转换为JSON值
    fn any_value_to_json_value(value: AnyValue) -> Result<serde_json::Value> {
        match value {
            AnyValue::Null => Ok(serde_json::Value::Null),
            AnyValue::Boolean(b) => Ok(serde_json::Value::Bool(b)),
            AnyValue::Int32(i) => Ok(serde_json::Value::Number(serde_json::Number::from(i))),
            AnyValue::Int64(i) => Ok(serde_json::Value::Number(serde_json::Number::from(i))),
            AnyValue::Float32(f) => {
                serde_json::Number::from_f64(f as f64)
                    .map(serde_json::Value::Number)
                    .ok_or_else(|| ConversionError::TypeConversion("无效的浮点数".to_string()).into())
            }
            AnyValue::Float64(f) => {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .ok_or_else(|| ConversionError::TypeConversion("无效的浮点数".to_string()).into())
            }
            AnyValue::String(s) => Ok(serde_json::Value::String(s.to_string())),
            AnyValue::StringOwned(s) => Ok(serde_json::Value::String(s.to_string())),
            AnyValue::Binary(b) => {
                // 将二进制数据编码为base64
                Ok(serde_json::Value::String(base64::encode(b)))
            }
            AnyValue::BinaryOwned(b) => {
                // 将二进制数据编码为base64
                Ok(serde_json::Value::String(base64::encode(&b)))
            }
            AnyValue::Datetime(dt, _, _) => {
                // 转换为ISO 8601格式字符串
                let datetime = chrono::DateTime::from_timestamp_millis(dt)
                    .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
                Ok(serde_json::Value::String(datetime.to_rfc3339()))
            }
            _ => Ok(serde_json::Value::String(format!("{:?}", value))),
        }
    }

    /// 将JSON值转换为AnyValue
    fn json_value_to_any_value(value: serde_json::Value) -> Result<AnyValue<'static>> {
        match value {
            serde_json::Value::Null => Ok(AnyValue::Null),
            serde_json::Value::Bool(b) => Ok(AnyValue::Boolean(b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                        Ok(AnyValue::Int32(i as i32))
                    } else {
                        Ok(AnyValue::Int64(i))
                    }
                } else if let Some(f) = n.as_f64() {
                    Ok(AnyValue::Float64(f))
                } else {
                    Err(ConversionError::TypeConversion("无效的数字格式".to_string()).into())
                }
            }
            serde_json::Value::String(s) => Ok(AnyValue::StringOwned(s.into())),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                // 复杂类型转换为字符串
                let json_str = serde_json::to_string(&value)
                    .map_err(|e| ConversionError::Serialization(e.to_string()))?;
                Ok(AnyValue::StringOwned(json_str.into()))
            }
        }
    }
}

// 添加base64依赖到Cargo.toml中用于二进制数据编码
mod base64 {
    pub fn encode(input: &[u8]) -> String {
        // 简单的base64编码实现
        // 在实际项目中应该使用base64 crate
        use std::collections::HashMap;
        
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        
        let mut result = String::new();
        let mut i = 0;
        
        while i < input.len() {
            let b1 = input[i];
            let b2 = if i + 1 < input.len() { input[i + 1] } else { 0 };
            let b3 = if i + 2 < input.len() { input[i + 2] } else { 0 };
            
            let bitmap = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);
            
            result.push(CHARS[((bitmap >> 18) & 63) as usize] as char);
            result.push(CHARS[((bitmap >> 12) & 63) as usize] as char);
            
            if i + 1 < input.len() {
                result.push(CHARS[((bitmap >> 6) & 63) as usize] as char);
            } else {
                result.push('=');
            }
            
            if i + 2 < input.len() {
                result.push(CHARS[(bitmap & 63) as usize] as char);
            } else {
                result.push('=');
            }
            
            i += 3;
        }
        
        result
    }
}