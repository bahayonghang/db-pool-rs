use crate::core::pool_manager::DistributedPoolManager;
use crate::core::types::{DatabaseConfig, DatabaseType, DeploymentMode, PoolConfig, TimeoutConfig, QueryParams, DatabaseValue, BatchOperation};
use crate::core::error::{DbPoolError, Result};
use crate::utils::dataframe::DataFrameConverter;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use pyo3_asyncio::tokio::future_into_py;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Python数据库连接池类
#[pyclass(name = "DatabasePool")]
pub struct PyDatabasePool {
    manager: Arc<DistributedPoolManager>,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyDatabasePool {
    /// 创建新的数据库连接池管理器
    #[new]
    #[pyo3(signature = (deployment_mode = "full"))]
    fn new(deployment_mode: &str) -> PyResult<Self> {
        let mode = match deployment_mode {
            "simple" => DeploymentMode::Simple,
            "balanced" => DeploymentMode::Balanced,
            "full" => DeploymentMode::Full,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("无效的部署模式: {}", deployment_mode)
            )),
        };

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("创建异步运行时失败: {}", e)
            ))?;

        let manager = Arc::new(DistributedPoolManager::new(mode));

        Ok(Self { manager, runtime })
    }

    /// 创建连接池
    #[pyo3(signature = (
        pool_id,
        db_type,
        host,
        port,
        database,
        username,
        password,
        min_connections = 5,
        max_connections = 50,
        acquire_timeout = 30,
        idle_timeout = 600,
        max_lifetime = 3600,
        auto_scaling = true,
        health_check_interval = 60,
        application_name = None
    ))]
    fn create_pool<'py>(
        &self,
        py: Python<'py>,
        pool_id: String,
        db_type: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
        min_connections: u32,
        max_connections: u32,
        acquire_timeout: u64,
        idle_timeout: u64,
        max_lifetime: u64,
        auto_scaling: bool,
        health_check_interval: u64,
        application_name: Option<String>,
    ) -> PyResult<&'py PyAny> {
        let manager = Arc::clone(&self.manager);
        
        // 创建配置
        let db_type_enum = match db_type.as_str() {
            "mssql" => DatabaseType::MSSQL,
            "postgresql" => DatabaseType::PostgreSQL,
            "redis" => DatabaseType::Redis,
            "sqlite" => DatabaseType::SQLite,
            "influxdb" => DatabaseType::InfluxDB,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("不支持的数据库类型: {}", db_type)
            )),
        };

        let config = DatabaseConfig {
            db_type: db_type_enum,
            host,
            port,
            database,
            username,
            password,
            pool_config: PoolConfig {
                min_connections,
                max_connections,
                acquire_timeout: Duration::from_secs(acquire_timeout),
                idle_timeout: Duration::from_secs(idle_timeout),
                max_lifetime: Duration::from_secs(max_lifetime),
                auto_scaling,
                scale_up_threshold: 0.8,
                scale_down_threshold: 0.3,
                health_check_interval: Duration::from_secs(health_check_interval),
            },
            ssl_config: None,
            timeout_config: TimeoutConfig::default(),
            application_name,
        };

        future_into_py::<_, PyObject>(py, async move {
            manager.create_pool(pool_id, config).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            Python::with_gil(|py| {
                Ok(py.None())
            })
        })
    }

    /// 执行查询
    #[pyo3(signature = (pool_id, sql, params = None))]
    fn query<'py>(
        &self,
        py: Python<'py>,
        pool_id: String,
        sql: String,
        params: Option<&PyDict>,
    ) -> PyResult<&'py PyAny> {
        let manager = Arc::clone(&self.manager);
        let query_params = if let Some(params_dict) = params {
            Some(Self::py_dict_to_query_params(params_dict)?)
        } else {
            None
        };

        future_into_py::<_, PyObject>(py, async move {
            let df = manager.execute_query(&pool_id, &sql, query_params).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            // 将Polars DataFrame转换为Python对象
            Python::with_gil(|py| {
                Self::polars_df_to_python(py, df)
            })
        })
    }

    /// 执行非查询操作
    #[pyo3(signature = (pool_id, sql, params = None))]
    fn execute<'py>(
        &self,
        py: Python<'py>,
        pool_id: String,
        sql: String,
        params: Option<&PyDict>,
    ) -> PyResult<&'py PyAny> {
        let manager = Arc::clone(&self.manager);
        let query_params = if let Some(params_dict) = params {
            Some(Self::py_dict_to_query_params(params_dict)?)
        } else {
            None
        };

        future_into_py::<_, PyObject>(py, async move {
            let result = manager.execute_non_query(&pool_id, &sql, query_params).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            Python::with_gil(|py| {
                Ok(result.into_py(py))
            })
        })
    }

    /// 批量执行操作
    fn execute_batch<'py>(
        &self,
        py: Python<'py>,
        pool_id: String,
        operations: &PyList,
    ) -> PyResult<&'py PyAny> {
        let manager = Arc::clone(&self.manager);
        let batch_ops = Self::py_list_to_batch_operations(operations)?;

        future_into_py::<_, PyObject>(py, async move {
            let results = manager.execute_batch(&pool_id, batch_ops).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            Python::with_gil(|py| {
                let py_results = PyList::empty(py);
                for result in results {
                    let result_dict = PyDict::new(py);
                    result_dict.set_item("affected_rows", result.affected_rows)?;
                    result_dict.set_item("execution_time_ms", result.execution_time.as_millis())?;
                    result_dict.set_item("error", result.error)?;
                    py_results.append(result_dict)?;
                }
                Ok(py_results.into())
            })
        })
    }

    /// 获取连接池状态
    fn get_pool_status<'py>(&self, py: Python<'py>, pool_id: String) -> PyResult<&'py PyAny> {
        let manager = Arc::clone(&self.manager);

        future_into_py::<_, PyObject>(py, async move {
            let status = manager.get_pool_status(&pool_id).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            Python::with_gil(|py| {
                let status_dict = PyDict::new(py);
                status_dict.set_item("pool_id", status.pool_id)?;
                status_dict.set_item("db_type", format!("{:?}", status.db_type))?;
                status_dict.set_item("total_connections", status.total_connections)?;
                status_dict.set_item("active_connections", status.active_connections)?;
                status_dict.set_item("idle_connections", status.idle_connections)?;
                status_dict.set_item("waiting_connections", status.waiting_connections)?;
                status_dict.set_item("is_healthy", status.is_healthy)?;
                status_dict.set_item("last_error", status.last_error)?;
                status_dict.set_item("uptime_seconds", status.uptime.as_secs())?;
                Ok(status_dict.into())
            })
        })
    }

    /// 获取连接池指标
    fn get_pool_metrics<'py>(&self, py: Python<'py>, pool_id: String) -> PyResult<&'py PyAny> {
        let manager = Arc::clone(&self.manager);

        future_into_py::<_, PyObject>(py, async move {
            let metrics = manager.get_pool_metrics(&pool_id).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            Python::with_gil(|py| {
                let metrics_dict = PyDict::new(py);
                metrics_dict.set_item("pool_id", metrics.pool_id)?;
                metrics_dict.set_item("queries_per_second", metrics.queries_per_second)?;
                metrics_dict.set_item("connection_utilization", metrics.connection_utilization)?;
                metrics_dict.set_item("avg_query_time_ms", metrics.avg_query_time.as_millis())?;
                metrics_dict.set_item("p99_query_time_ms", metrics.p99_query_time.as_millis())?;
                metrics_dict.set_item("error_rate", metrics.error_rate)?;
                metrics_dict.set_item("total_queries", metrics.total_queries)?;
                metrics_dict.set_item("total_errors", metrics.total_errors)?;
                metrics_dict.set_item("cache_hit_rate", metrics.cache_hit_rate)?;
                Ok(metrics_dict.into())
            })
        })
    }

    /// 列出所有连接池
    fn list_pools(&self) -> PyResult<Vec<String>> {
        Ok(self.manager.list_pools())
    }

    /// 移除连接池
    fn remove_pool<'py>(&self, py: Python<'py>, pool_id: String) -> PyResult<&'py PyAny> {
        let manager = Arc::clone(&self.manager);

        future_into_py::<_, PyObject>(py, async move {
            manager.remove_pool(&pool_id).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            Python::with_gil(|py| {
                Ok(py.None())
            })
        })
    }

    /// 健康检查
    fn health_check<'py>(&self, py: Python<'py>, pool_id: String) -> PyResult<&'py PyAny> {
        let manager = Arc::clone(&self.manager);

        future_into_py::<_, PyObject>(py, async move {
            let result = manager.health_check(&pool_id).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            Python::with_gil(|py| {
                Ok(result.into_py(py))
            })
        })
    }

    /// 获取版本信息
    #[staticmethod]
    fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// 获取支持的数据库类型
    #[staticmethod]
    fn supported_databases() -> Vec<String> {
        vec![
            "mssql".to_string(),
            "postgresql".to_string(),
            "redis".to_string(),
            "sqlite".to_string(),
            "influxdb".to_string(),
        ]
    }
}

impl PyDatabasePool {
    /// 将Python字典转换为查询参数
    fn py_dict_to_query_params(py_dict: &PyDict) -> PyResult<QueryParams> {
        let mut params = HashMap::new();

        for (key, value) in py_dict.iter() {
            let key_str = key.extract::<String>()?;
            let db_value = Self::py_any_to_database_value(value)?;
            params.insert(key_str, db_value);
        }

        Ok(params)
    }

    /// 将Python值转换为数据库值
    fn py_any_to_database_value(value: &PyAny) -> PyResult<DatabaseValue> {
        if value.is_none() {
            Ok(DatabaseValue::Null)
        } else if let Ok(b) = value.extract::<bool>() {
            Ok(DatabaseValue::Bool(b))
        } else if let Ok(i) = value.extract::<i32>() {
            Ok(DatabaseValue::I32(i))
        } else if let Ok(i) = value.extract::<i64>() {
            Ok(DatabaseValue::I64(i))
        } else if let Ok(f) = value.extract::<f32>() {
            Ok(DatabaseValue::F32(f))
        } else if let Ok(f) = value.extract::<f64>() {
            Ok(DatabaseValue::F64(f))
        } else if let Ok(s) = value.extract::<String>() {
            Ok(DatabaseValue::String(s))
        } else if let Ok(b) = value.extract::<Vec<u8>>() {
            Ok(DatabaseValue::Bytes(b))
        } else {
            // 默认转换为字符串
            let s = value.str()?.extract::<String>()?;
            Ok(DatabaseValue::String(s))
        }
    }

    /// 将Python列表转换为批量操作
    fn py_list_to_batch_operations(py_list: &PyList) -> PyResult<Vec<BatchOperation>> {
        let mut operations = Vec::new();

        for item in py_list.iter() {
            if let Ok(sql) = item.extract::<String>() {
                // 简单字符串格式
                operations.push(BatchOperation {
                    sql,
                    params: None,
                });
            } else if let Ok(dict) = item.downcast::<PyDict>() {
                // 字典格式 {"sql": "...", "params": {...}}
                let sql = dict.get_item("sql")?
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("缺少sql字段"))?
                    .extract::<String>()?;

                let params = if let Ok(Some(params_item)) = dict.get_item("params") {
                    if let Ok(params_dict) = params_item.downcast::<PyDict>() {
                        Some(Self::py_dict_to_query_params(params_dict)?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                operations.push(BatchOperation { sql, params });
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "批量操作必须是字符串或字典格式"
                ));
            }
        }

        Ok(operations)
    }

    /// 将Polars DataFrame转换为Python对象
    fn polars_df_to_python(py: Python, df: polars::frame::DataFrame) -> PyResult<PyObject> {
        // 将DataFrame转换为字典格式
        let mut data = HashMap::new();
        
        for column_name in df.get_column_names() {
            let series = df.column(column_name)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            let mut column_data = Vec::new();
            for i in 0..series.len() {
                let value = series.get(i)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
                
                let py_value = Self::any_value_to_python(py, value)?;
                column_data.push(py_value);
            }
            
            data.insert(column_name.to_string(), column_data);
        }

        // 创建Python字典
        let result_dict = PyDict::new(py);
        for (col_name, col_data) in data {
            let py_list = PyList::new(py, col_data);
            result_dict.set_item(col_name, py_list)?;
        }

        Ok(result_dict.into())
    }

    /// 将Polars AnyValue转换为Python对象
    fn any_value_to_python(py: Python, value: polars::prelude::AnyValue) -> PyResult<PyObject> {
        use polars::prelude::AnyValue;

        match value {
            AnyValue::Null => Ok(py.None()),
            AnyValue::Boolean(b) => Ok(b.into_py(py)),
            AnyValue::Int32(i) => Ok(i.into_py(py)),
            AnyValue::Int64(i) => Ok(i.into_py(py)),
            AnyValue::Float32(f) => Ok(f.into_py(py)),
            AnyValue::Float64(f) => Ok(f.into_py(py)),
            AnyValue::String(s) => Ok(s.into_py(py)),
            AnyValue::StringOwned(s) => Ok(s.to_string().into_py(py)),
            AnyValue::Binary(b) => Ok(b.into_py(py)),
            AnyValue::BinaryOwned(b) => Ok(b.into_py(py)),
            AnyValue::Datetime(dt, _, _) => {
                // 转换为Python datetime
                let timestamp = dt as f64 / 1000.0; // 转换为秒
                let datetime_module = py.import("datetime")?;
                let datetime_class = datetime_module.getattr("datetime")?;
                let from_timestamp = datetime_class.getattr("fromtimestamp")?;
                from_timestamp.call1((timestamp,))?.extract()
            }
            _ => {
                // 其他类型转换为字符串
                Ok(format!("{:?}", value).into_py(py))
            }
        }
    }
}